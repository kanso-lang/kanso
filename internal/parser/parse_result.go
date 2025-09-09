package parser

import "kanso/internal/ast"

// ParseResult contains the full parsing result including metadata
type ParseResult struct {
	Contract        *ast.Contract
	ParseErrors     []ParseError
	ScanErrors      []ScanError
	MetadataVisitor *ast.MetadataVisitor
}

// ParseSourceWithMetadata parses source code and returns enhanced result with metadata
func ParseSourceWithMetadata(path string, source string) *ParseResult {
	scanner := NewScanner(source)
	tokens := scanner.ScanTokens()

	parser := NewParser(path, tokens)
	contract := parser.ParseContract()

	var mv *ast.MetadataVisitor
	// Assign metadata to all AST nodes
	if contract != nil {
		mv = ast.NewMetadataVisitor(source)
		// Assign metadata to leading comments
		for _, item := range contract.LeadingComments {
			mv.AssignMetadata(item, 0) // 0 = no parent
		}
		// Assign metadata to contract items
		for _, item := range contract.Items {
			mv.AssignMetadata(item, 0) // 0 = no parent
		}
	}

	return &ParseResult{
		Contract:        contract,
		ParseErrors:     parser.errors,
		ScanErrors:      scanner.errors,
		MetadataVisitor: mv,
	}
}

// GetSourceMapping returns source-to-bytecode mapping for DAP server
func (pr *ParseResult) GetSourceMapping() map[uint32]ast.Position {
	if pr.Contract == nil || pr.MetadataVisitor == nil {
		return nil
	}

	nodes := ast.CollectAllNodes(pr.Contract.Items[0])
	return ast.GetSourceMapping(nodes)
}

// GetReverseMapping returns bytecode-to-source mapping for DAP server
func (pr *ParseResult) GetReverseMapping() map[ast.Position][]uint32 {
	if pr.Contract == nil || pr.MetadataVisitor == nil {
		return nil
	}

	nodes := ast.CollectAllNodes(pr.Contract.Items[0])
	return ast.GetReverseMapping(nodes)
}

// FindNodeByPosition finds a node at a specific position (for DAP server)
func (pr *ParseResult) FindNodeByPosition(pos ast.Position) *ast.Metadata {
	if pr.MetadataVisitor == nil {
		return nil
	}
	return pr.MetadataVisitor.FindNodeByPosition(pos)
}

// GetDebugInfo returns debugging information about the parse result
func (pr *ParseResult) GetDebugInfo() string {
	if pr.MetadataVisitor == nil {
		return "No metadata available"
	}
	return pr.MetadataVisitor.PrintDebugInfo()
}
