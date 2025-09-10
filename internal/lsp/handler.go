package lsp

import (
	"fmt"
	"kanso/internal/ast"
	"kanso/internal/parser"
	"kanso/internal/semantic"
	"log"
	"net/url"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"

	"github.com/tliron/glsp"
	protocol "github.com/tliron/glsp/protocol_3_16"
)

// SemanticTokenTypes defines the vocabulary for syntax highlighting in IDEs.
// This establishes a contract with editors about what kinds of symbols we can highlight,
// enabling rich visual feedback for developers working with Kanso contracts.
var SemanticTokenTypes = []string{
	"namespace",
	"type",
	"typeParameter",
	"function",
	"variable",
	"parameter",
	"property",
	"keyword",
	"number",
	"operator",
	"modifier",
}

// SemanticTokenModifiers provide contextual information about symbols.
// These modifiers help IDEs apply appropriate styling (e.g., italic for deprecated,
// bold for declarations) to enhance code readability.
var SemanticTokenModifiers = []string{
	"declaration",
	"definition",
	"readonly",
	"static",
	"deprecated",
	"abstract",
}

// KansoHandler maintains the state needed for intelligent IDE support.
// We cache parsed content and ASTs to avoid expensive re-parsing on every request,
// which dramatically improves responsiveness during development.
type KansoHandler struct {
	mu             sync.RWMutex                        // Protects concurrent access from multiple editor operations
	content        map[string]string                   // Raw file content cache
	asts           map[string]*ast.Contract            // Parsed AST cache for quick access
	semanticErrors map[string][]semantic.SemanticError // Enables incremental semantic analysis updates
}

// NewKansoHandler creates and returns a new KansoHandler instance
func NewKansoHandler() *KansoHandler {
	return &KansoHandler{
		content:        make(map[string]string),
		asts:           make(map[string]*ast.Contract),
		semanticErrors: make(map[string][]semantic.SemanticError),
	}
}

// Initialize responds to the LSP client's initialize request and advertises the server's capabilities
func (h *KansoHandler) Initialize(ctx *glsp.Context, params *protocol.InitializeParams) (any, error) {

	return &protocol.InitializeResult{
		Capabilities: protocol.ServerCapabilities{
			TextDocumentSync: &protocol.TextDocumentSyncOptions{
				OpenClose: ptrBool(true),                                  // We need open/close events to manage our cache lifecycle
				Change:    ptrSyncKind(protocol.TextDocumentSyncKindFull), // Full sync is simpler and more reliable than incremental
			},
			CompletionProvider: &protocol.CompletionOptions{
				ResolveProvider: ptrBool(false), // Keep completion fast by providing all details upfront
			},
			SemanticTokensProvider: &protocol.SemanticTokensOptions{
				Legend: protocol.SemanticTokensLegend{
					TokenTypes:     SemanticTokenTypes,
					TokenModifiers: SemanticTokenModifiers,
				},
				Full: ptrBool(true), // Full-document analysis gives us complete semantic context
			},
		},
	}, nil
}

// Initialized is called after the client receives the server's capabilities and completes initialization
func (h *KansoHandler) Initialized(ctx *glsp.Context, params *protocol.InitializedParams) error {
	return nil
}

// Shutdown handles the LSP shutdown request
func (h *KansoHandler) Shutdown(ctx *glsp.Context) error {
	return nil
}

// TextDocumentDidOpen handles file open notifications from the editor
func (h *KansoHandler) TextDocumentDidOpen(ctx *glsp.Context, params *protocol.DidOpenTextDocumentParams) error {

	path, err := uriToPath(params.TextDocument.URI)
	if err != nil {
		return fmt.Errorf("failed to convert URI %s: %w", params.TextDocument.URI, err)
	}

	// Cache the initial document content from VS Code
	h.mu.Lock()
	h.content[path] = params.TextDocument.Text
	h.mu.Unlock()

	diagnostics, err := h.updateAST(params.TextDocument.URI)
	if err != nil {
		return fmt.Errorf("failed to update AST: %w", err)
	}

	sendDiagnosticNotification(ctx, params.TextDocument.URI, diagnostics)
	return nil
}

// TextDocumentDidClose handles file close notifications from the editor
func (h *KansoHandler) TextDocumentDidClose(context *glsp.Context, params *protocol.DidCloseTextDocumentParams) error {

	path, err := uriToPath(params.TextDocument.URI)
	if err != nil {
		return fmt.Errorf("failed to convert URI %s: %w", params.TextDocument.URI, err)
	}

	// Clean up cached data to prevent memory leaks
	h.mu.Lock()
	defer h.mu.Unlock()
	delete(h.content, path)
	delete(h.asts, path)
	delete(h.semanticErrors, path)

	return nil
}

// TextDocumentDidChange handles file change notifications from the editor
func (h *KansoHandler) TextDocumentDidChange(ctx *glsp.Context, params *protocol.DidChangeTextDocumentParams) error {

	path, err := uriToPath(params.TextDocument.URI)
	if err != nil {
		return fmt.Errorf("failed to convert URI %s: %w", params.TextDocument.URI, err)
	}

	// Extract and cache the updated content from VS Code
	newContent := h.applyTextChanges(path, params.ContentChanges)
	h.mu.Lock()
	h.content[path] = newContent
	h.mu.Unlock()

	diagnostics, err := h.updateAST(params.TextDocument.URI)
	if err != nil {
		return fmt.Errorf("failed to update AST: %w", err)
	}

	sendDiagnosticNotification(ctx, params.TextDocument.URI, diagnostics)
	return nil
}

// TextDocumentCompletion accelerates development by suggesting contextually relevant code.
// Rather than forcing developers to remember exact syntax, we provide smart suggestions
// based on the current contract structure and Kanso language constructs.
func (h *KansoHandler) TextDocumentCompletion(ctx *glsp.Context, params *protocol.CompletionParams) (interface{}, error) {
	rawURI := params.TextDocument.URI
	path, err := uriToPath(rawURI)
	if err != nil {
		// Return empty list rather than error to maintain editor responsiveness
		return &protocol.CompletionList{IsIncomplete: false, Items: []protocol.CompletionItem{}}, nil
	}

	contract, err := h.getOrUpdateAST(ctx, path, rawURI)
	if err != nil || contract == nil {
		// Graceful degradation: provide basic completions even with parse errors
		return &protocol.CompletionList{IsIncomplete: false, Items: []protocol.CompletionItem{}}, nil
	}

	completionItems := h.generateCompletionItems(contract, params.Position, path)

	return &protocol.CompletionList{
		IsIncomplete: false,
		Items:        completionItems,
	}, nil
}

// generateCompletionItems builds suggestions that reduce cognitive load for developers.
// We prioritize common patterns and contract-specific constructs to help developers
// write correct Kanso code faster, especially when learning the language.
func (h *KansoHandler) generateCompletionItems(contract *ast.Contract, position protocol.Position, filePath string) []protocol.CompletionItem {
	var items []protocol.CompletionItem

	// Get the current line content to analyze context from the current editing session
	currentLine := h.getLineContent(filePath, int(position.Line))

	// Detect if we're completing struct field access (e.g., "State." or "myVar.")
	if structContext := h.detectStructFieldContext(currentLine, int(position.Character)); structContext != nil {
		return h.generateStructFieldCompletions(contract, *structContext)
	}

	// Provide Kanso-specific constructs as snippets to reduce boilerplate typing
	keywords := []struct {
		label, detail, insertText string
		kind                      protocol.CompletionItemKind
	}{
		{"let", "immutable variable declaration", "let ${1:name} = ${2:value};", protocol.CompletionItemKindKeyword},
		{"let mut", "mutable variable declaration", "let mut ${1:name} = ${2:value};", protocol.CompletionItemKindKeyword},
		{"fn", "function declaration", "fn ${1:name}() {\n\t${2:// body}\n}", protocol.CompletionItemKindKeyword},
		{"ext fn", "external function", "ext fn ${1:name}() -> ${2:Type} {\n\t${3:// body}\n}", protocol.CompletionItemKindKeyword},
		{"require!", "assertion macro", "require!(${1:condition}, ${2:error});", protocol.CompletionItemKindFunction},
		{"#[storage]", "storage struct attribute", "#[storage]\nstruct ${1:Name} {\n\t${2:field}: ${3:Type},\n}", protocol.CompletionItemKindKeyword},
		{"#[event]", "event struct attribute", "#[event]\nstruct ${1:Name} {\n\t${2:field}: ${3:Type},\n}", protocol.CompletionItemKindKeyword},
		{"#[create]", "constructor attribute", "#[create]\nfn create() writes ${1:State} {\n\t${2:// initialization}\n}", protocol.CompletionItemKindKeyword},
	}

	for _, kw := range keywords {
		items = append(items, protocol.CompletionItem{
			Label:      kw.label,
			Kind:       &kw.kind,
			Detail:     &kw.detail,
			InsertText: &kw.insertText,
			InsertTextFormat: func() *protocol.InsertTextFormat {
				f := protocol.InsertTextFormatSnippet
				return &f
			}(),
		})
	}

	// Suggest struct fields to prevent typos and enable quick field access.
	// This is especially valuable for storage structs with many fields.
	for _, item := range contract.Items {
		if structDef, ok := item.(*ast.Struct); ok {
			for _, structItem := range structDef.Items {
				if field, ok := structItem.(*ast.StructField); ok {
					fieldKind := protocol.CompletionItemKindField
					detail := fmt.Sprintf("field of struct %s", structDef.Name.Value)
					items = append(items, protocol.CompletionItem{
						Label:  field.Name.Value,
						Kind:   &fieldKind,
						Detail: &detail,
					})
				}
			}
		}
	}

	// Include contract functions to facilitate internal function calls and refactoring.
	// Distinguishing external vs internal helps developers understand call semantics.
	for _, item := range contract.Items {
		if funcDef, ok := item.(*ast.Function); ok {
			funcKind := protocol.CompletionItemKindFunction
			detail := "contract function"
			if funcDef.External {
				detail = "external function"
			}
			items = append(items, protocol.CompletionItem{
				Label:  funcDef.Name.Value,
				Kind:   &funcKind,
				Detail: &detail,
			})
		}
	}

	return items
}

// TextDocumentSemanticTokensFull handles semantic token requests for the entire document
func (h *KansoHandler) TextDocumentSemanticTokensFull(ctx *glsp.Context, params *protocol.SemanticTokensParams) (*protocol.SemanticTokens, error) {

	rawURI := params.TextDocument.URI

	path, err := uriToPath(rawURI)
	if err != nil {
		return nil, fmt.Errorf("failed to convert URI %s: %w", rawURI, err)
	}

	ast, err := h.getOrUpdateAST(ctx, path, rawURI)
	if err != nil {
		return nil, err
	}

	// Walk the AST and collect semantic tokens
	tokens := collectSemanticTokens(ast)

	var data []uint32
	var prevLine, prevStart uint32

	// Encode tokens into LSP wire format (using delta-line, delta-start compression)
	for _, token := range tokens {
		deltaLine := token.Line - prevLine
		var deltaStart uint32
		if deltaLine == 0 {
			deltaStart = token.StartChar - prevStart
		} else {
			deltaStart = token.StartChar
		}

		// Append the encoded semantic token entry
		data = append(data, deltaLine, deltaStart, token.Length, uint32(token.TokenType), uint32(token.TokenModifiers))

		prevLine = token.Line
		prevStart = token.StartChar
	}

	return &protocol.SemanticTokens{
		Data: data,
	}, nil
}

// Helper functions for creating LSP protocol pointers
func ptrSeverity(s protocol.DiagnosticSeverity) *protocol.DiagnosticSeverity {
	return &s
}

func ptrString(s string) *string {
	return &s
}

func (h *KansoHandler) getOrUpdateAST(ctx *glsp.Context, path string, rawURI protocol.DocumentUri) (*ast.Contract, error) {
	// Try to use cached AST first to avoid expensive re-parsing on every request
	h.mu.RLock()
	ast, ok := h.asts[path]
	h.mu.RUnlock()

	if !ok {
		// Cache miss: parse the file and update diagnostics
		diagnostic, err := h.updateAST(rawURI)
		if err != nil {
			return nil, err
		}

		h.mu.RLock()
		ast = h.asts[path]
		h.mu.RUnlock()

		// Always provide immediate feedback to developer about syntax/semantic errors
		// Send diagnostics even if empty to clear previous errors
		sendDiagnosticNotification(ctx, rawURI, diagnostic)
	}

	return ast, nil
}

func (h *KansoHandler) updateAST(rawURI protocol.DocumentUri) ([]protocol.Diagnostic, error) {
	path, err := uriToPath(rawURI)
	if err != nil {
		return nil, fmt.Errorf("failed to convert URI %s: %w", rawURI, err)
	}

	// Get file content from VS Code cache or fallback to disk
	contentBytes, err := h.getFileContent(path)
	if err != nil {
		return nil, err
	}

	contract, parserErrors, scannerErrors := parser.ParseSource(path, string(contentBytes))

	// Convert parse and scanner errors to LSP diagnostics
	var allDiagnostics []protocol.Diagnostic
	if len(parserErrors) > 0 || len(scannerErrors) > 0 {
		allDiagnostics = append(allDiagnostics, ConvertParseErrors(parserErrors)...)
		allDiagnostics = append(allDiagnostics, ConvertScanErrors(scannerErrors)...)

		// If we have critical parse errors, return early with just syntax errors
		if contract == nil {
			return allDiagnostics, nil
		}
		// Otherwise, continue to semantic analysis and add those errors too
	}

	// Run semantic analysis to catch logical errors that parsing alone cannot detect.
	// This provides developers with immediate feedback about type mismatches,
	// undefined references, and contract-specific constraint violations.
	if contract != nil {
		analyzer := semantic.NewAnalyzer()
		semanticErrors := analyzer.Analyze(contract)

		for _, semErr := range semanticErrors {
			allDiagnostics = append(allDiagnostics, protocol.Diagnostic{
				Range: protocol.Range{
					Start: protocol.Position{
						Line:      uint32(semErr.Position.Line - 1), // Convert to LSP's 0-based indexing
						Character: uint32(semErr.Position.Column - 1),
					},
					End: protocol.Position{
						Line:      uint32(semErr.Position.Line - 1),
						Character: uint32(semErr.Position.Column + 10), // Rough error span estimation
					},
				},
				Severity: ptrSeverity(protocol.DiagnosticSeverityError),
				Source:   ptrString("kanso-semantic"),
				Message:  semErr.Message,
			})
		}

		// Cache semantic errors for potential incremental analysis optimizations
		h.mu.Lock()
		h.semanticErrors[path] = semanticErrors
		h.mu.Unlock()
	}

	// Update all caches atomically to maintain consistency
	h.mu.Lock()
	h.content[path] = string(contentBytes)
	h.asts[path] = contract
	h.mu.Unlock()

	return allDiagnostics, nil
}

// uriToPath handles the impedance mismatch between URI format used by LSP
// and local filesystem paths. This cross-platform conversion is essential
// because different operating systems have different path conventions.
func uriToPath(rawURI string) (string, error) {
	u, err := url.Parse(rawURI)
	if err != nil {
		return "", fmt.Errorf("invalid URI %s: %w", rawURI, err)
	}

	path := u.Path

	// Windows paths in URIs have an extra leading slash that must be removed
	if runtime.GOOS == "windows" && strings.HasPrefix(path, "/") && len(path) > 3 && path[2] == ':' {
		path = path[1:]
	}

	return filepath.FromSlash(path), nil
}

func sendDiagnosticNotification(ctx *glsp.Context, uri protocol.URI, diagnostics []protocol.Diagnostic) {
	// Ensure we have a valid diagnostics slice, even if empty
	if diagnostics == nil {
		diagnostics = []protocol.Diagnostic{}
	}

	// Send diagnostics to editor

	// Graceful handling prevents test failures when LSP context is not fully initialized.
	// This defensive approach allows both production and testing scenarios to work seamlessly.
	defer func() {
		if r := recover(); r != nil {
			log.Printf("Failed to send diagnostics notification (likely in test environment): %v", r)
		}
	}()

	ctx.Notify(protocol.ServerTextDocumentPublishDiagnostics, &protocol.PublishDiagnosticsParams{
		URI:         uri,
		Diagnostics: diagnostics,
	})
}

func ptrBool(b bool) *bool {
	return &b
}

func ptrSyncKind(k protocol.TextDocumentSyncKind) *protocol.TextDocumentSyncKind {
	return &k
}

// StructFieldContext contains information about struct field access context
type StructFieldContext struct {
	StructName    string // Name of the struct being accessed (e.g., "State")
	IsKnownStruct bool   // Whether we can find this struct in the contract
}

// detectStructFieldContext analyzes the current line to determine if we're completing struct field access
func (h *KansoHandler) detectStructFieldContext(line string, cursorPos int) *StructFieldContext {
	if cursorPos > len(line) {
		cursorPos = len(line)
	}

	// Look for pattern like "StructName." where cursor is right after the dot
	beforeCursor := line[:cursorPos]

	// Find the last dot and extract what comes before it
	lastDotIndex := strings.LastIndex(beforeCursor, ".")
	if lastDotIndex == -1 {
		return nil
	}

	// Extract the potential struct name (everything between the last whitespace/delimiter and the dot)
	beforeDot := beforeCursor[:lastDotIndex]

	// Find the start of the identifier (look backwards for word boundary)
	structStart := lastDotIndex - 1
	for structStart >= 0 && (isAlphanumeric(beforeDot[structStart]) || beforeDot[structStart] == '_') {
		structStart--
	}
	structStart++ // Move to the first character of the identifier

	if structStart >= lastDotIndex {
		return nil // No valid identifier found
	}

	structName := beforeDot[structStart:]

	return &StructFieldContext{
		StructName:    structName,
		IsKnownStruct: false, // Will be determined in generateStructFieldCompletions
	}
}

// generateStructFieldCompletions returns only struct field completions for the given context
func (h *KansoHandler) generateStructFieldCompletions(contract *ast.Contract, context StructFieldContext) []protocol.CompletionItem {
	var items []protocol.CompletionItem

	// Find the struct definition in the contract
	var targetStruct *ast.Struct
	for _, item := range contract.Items {
		if structDef, ok := item.(*ast.Struct); ok && structDef.Name.Value == context.StructName {
			targetStruct = structDef
			break
		}
	}

	if targetStruct == nil {
		return items // Return empty if struct not found
	}

	// Add only the fields of the matched struct
	for _, structItem := range targetStruct.Items {
		if field, ok := structItem.(*ast.StructField); ok {
			fieldKind := protocol.CompletionItemKindField
			detail := fmt.Sprintf("field of struct %s", targetStruct.Name.Value)
			items = append(items, protocol.CompletionItem{
				Label:  field.Name.Value,
				Kind:   &fieldKind,
				Detail: &detail,
			})
		}
	}

	return items
}

// getLineContent retrieves the content of a specific line from the cached file content
func (h *KansoHandler) getLineContent(path string, lineNumber int) string {
	h.mu.RLock()
	defer h.mu.RUnlock()

	content, exists := h.content[path]
	if !exists {
		return ""
	}

	lines := strings.Split(content, "\n")
	if lineNumber < 0 || lineNumber >= len(lines) {
		return ""
	}

	return lines[lineNumber]
}

// isAlphanumeric checks if a character is alphanumeric
func isAlphanumeric(c byte) bool {
	return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')
}

// applyTextChanges applies document changes to get the new content.
// Since we use TextDocumentSyncKindFull, we expect complete document replacements.
func (h *KansoHandler) applyTextChanges(path string, changes []any) string {
	if len(changes) == 0 {
		return h.getCurrentContent(path)
	}

	// Handle the protocol type for full document sync
	if change, ok := changes[0].(protocol.TextDocumentContentChangeEventWhole); ok {
		return change.Text
	}

	// Fallback: try to extract text from generic map structure
	if changeMap, ok := changes[0].(map[string]any); ok {
		if text, exists := changeMap["text"]; exists {
			if textStr, ok := text.(string); ok {
				return textStr
			}
		}
	}

	// Last resort: return current cached content
	return h.getCurrentContent(path)
}

// getCurrentContent safely retrieves the current cached content for a file
func (h *KansoHandler) getCurrentContent(path string) string {
	h.mu.RLock()
	defer h.mu.RUnlock()
	return h.content[path]
}

// getFileContent retrieves file content from VS Code cache or fallback to disk
func (h *KansoHandler) getFileContent(path string) ([]byte, error) {
	h.mu.RLock()
	content, hasCached := h.content[path]
	h.mu.RUnlock()

	if hasCached {
		return []byte(content), nil
	}

	// Fallback to reading from disk if no cached content
	diskContent, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read file %s: %w", path, err)
	}

	return diskContent, nil
}

// GenerateCompletionItemsForTesting is a testing helper that simulates completion without LSP context
func (h *KansoHandler) GenerateCompletionItemsForTesting(contract *ast.Contract, line, character int, content string) []protocol.CompletionItem {
	// Store the content temporarily for testing
	testPath := "test.ka"
	h.mu.Lock()
	h.content[testPath] = content
	h.mu.Unlock()
	defer func() {
		h.mu.Lock()
		delete(h.content, testPath)
		h.mu.Unlock()
	}()

	position := protocol.Position{
		Line:      uint32(line),
		Character: uint32(character),
	}

	return h.generateCompletionItems(contract, position, testPath)
}

// UpdateASTForTesting exposes updateAST for testing purposes
func (h *KansoHandler) UpdateASTForTesting(uri protocol.DocumentUri) ([]protocol.Diagnostic, error) {
	return h.updateAST(uri)
}
