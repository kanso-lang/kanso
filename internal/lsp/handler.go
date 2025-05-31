package lsp

import (
	"fmt"
	"kanso/grammar"
	"kanso/internal/parser"
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

// Define the set of supported semantic token types (as required by the LSP spec)
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

// Define the set of supported semantic token modifiers (for extra tagging like declaration, readonly, etc.)
var SemanticTokenModifiers = []string{
	"declaration",
	"definition",
	"readonly",
	"static",
	"deprecated",
	"abstract",
}

// KansoHandler implements the LSP server handlers for the Kanso language
type KansoHandler struct {
	mu      sync.RWMutex
	content map[string]string
	asts    map[string]*grammar.AST
}

// NewKansoHandler creates and returns a new KansoHandler instance
func NewKansoHandler() *KansoHandler {
	return &KansoHandler{
		content: make(map[string]string),
		asts:    make(map[string]*grammar.AST),
	}
}

// Utility: return a pointer to a bool value
func ptrBool(b bool) *bool {
	return &b
}

// Utility: return a pointer to a TextDocumentSyncKind value
func ptrSyncKind(k protocol.TextDocumentSyncKind) *protocol.TextDocumentSyncKind {
	return &k
}

// Initialize responds to the LSP client's initialize request and advertises the server's capabilities
func (h *KansoHandler) Initialize(ctx *glsp.Context, params *protocol.InitializeParams) (any, error) {
	log.Println("LSP Initialize called")

	return &protocol.InitializeResult{
		Capabilities: protocol.ServerCapabilities{
			TextDocumentSync: &protocol.TextDocumentSyncOptions{
				OpenClose: ptrBool(true),                                         // notify on open/close events
				Change:    ptrSyncKind(protocol.TextDocumentSyncKindIncremental), // track incremental changes
			},
			CompletionProvider: &protocol.CompletionOptions{
				ResolveProvider: ptrBool(false), // no additional detail resolution yet
			},
			SemanticTokensProvider: &protocol.SemanticTokensOptions{
				Legend: protocol.SemanticTokensLegend{
					TokenTypes:     SemanticTokenTypes,
					TokenModifiers: SemanticTokenModifiers,
				},
				Full: ptrBool(true), // support full-document semantic token requests
			},
		},
	}, nil
}

// Initialized is called after the client receives the server's capabilities and completes initialization
func (h *KansoHandler) Initialized(ctx *glsp.Context, params *protocol.InitializedParams) error {
	log.Println("LSP Initialized")
	return nil
}

// Shutdown handles the LSP shutdown request
func (h *KansoHandler) Shutdown(ctx *glsp.Context) error {
	log.Println("LSP Shutdown")
	return nil
}

// SetTrace handles trace setting (not used currently)
func (h *KansoHandler) SetTrace(ctx *glsp.Context, params *protocol.SetTraceParams) error {
	return nil
}

// TextDocumentDidOpen handles file open notifications from the editor
func (h *KansoHandler) TextDocumentDidOpen(ctx *glsp.Context, params *protocol.DidOpenTextDocumentParams) error {
	log.Printf("Opened file: %s\n", params.TextDocument.URI)

	err := h.updateAST(params.TextDocument.URI)
	if err != nil {
		return fmt.Errorf("Failed to update AST:  %w", err)
	}

	return nil
}

// TextDocumentDidClose handles file close notifications from the editor
func (h *KansoHandler) TextDocumentDidClose(context *glsp.Context, params *protocol.DidCloseTextDocumentParams) error {
	log.Printf("Closed file: %s\n", params.TextDocument.URI)

	rawURI := params.TextDocument.URI

	path, err := uriToPath(rawURI)
	if err != nil {
		return fmt.Errorf("failed to convert URI %s: %w", rawURI, err)
	}

	h.mu.Lock()
	defer h.mu.Unlock()
	delete(h.content, path)
	delete(h.asts, path)

	return nil
}

// TextDocumentDidChange handles file change notifications from the editor
func (h *KansoHandler) TextDocumentDidChange(ctx *glsp.Context, params *protocol.DidChangeTextDocumentParams) error {
	log.Printf("Changed file: %s\n", params.TextDocument.URI)

	err := h.updateAST(params.TextDocument.URI)
	if err != nil {
		return fmt.Errorf("Failed to update AST:  %w", err)
	}

	return nil
}

// TextDocumentCompletion handles completion requests (currently returns empty list)
func (h *KansoHandler) TextDocumentCompletion(ctx *glsp.Context, params *protocol.CompletionParams) (interface{}, error) {
	// You could extend this to provide Kanso-specific completions
	return &protocol.CompletionList{
		IsIncomplete: false,
		Items:        []protocol.CompletionItem{},
	}, nil
}

// TextDocumentSemanticTokensFull handles semantic token requests for the entire document
func (h *KansoHandler) TextDocumentSemanticTokensFull(ctx *glsp.Context, params *protocol.SemanticTokensParams) (*protocol.SemanticTokens, error) {
	log.Println("TextDocumentSemanticTokensFull called for:", params.TextDocument.URI)

	rawURI := params.TextDocument.URI

	path, err := uriToPath(rawURI)
	if err != nil {
		return nil, fmt.Errorf("failed to convert URI %s: %w", rawURI, err)
	}

	ast, err := h.getOrUpdateAST(path, rawURI)
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

func (h *KansoHandler) getOrUpdateAST(path string, rawURI protocol.DocumentUri) (*grammar.AST, error) {
	h.mu.RLock()
	ast, ok := h.asts[path]
	h.mu.RUnlock()

	if !ok {
		if err := h.updateAST(rawURI); err != nil {
			return nil, err
		}

		h.mu.RLock()
		ast = h.asts[path]
		h.mu.RUnlock()
	}

	return ast, nil
}

func (h *KansoHandler) updateAST(rawURI protocol.DocumentUri) error {
	path, err := uriToPath(rawURI)
	if err != nil {
		return fmt.Errorf("failed to convert URI %s: %w", rawURI, err)
	}

	content, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("failed to read file %s: %w", path, err)
	}

	ast, err := parser.ParseSource(path, string(content))
	if err != nil {
		return fmt.Errorf("failed to parse file %s: %w", path, err)
	}

	h.mu.Lock()
	h.content[path] = string(content)
	h.asts[path] = ast
	h.mu.Unlock()

	return nil
}

// Convert URI to platform-local file path
func uriToPath(rawURI string) (string, error) {
	u, err := url.Parse(rawURI)
	if err != nil {
		return "", fmt.Errorf("invalid URI %s: %w", rawURI, err)
	}

	path := u.Path

	// On Windows, remove leading slash (e.g., /C:/...) → C:/...
	if runtime.GOOS == "windows" && strings.HasPrefix(path, "/") && len(path) > 3 && path[2] == ':' {
		path = path[1:]
	}

	// Normalize to platform-specific separators
	return filepath.FromSlash(path), nil
}
