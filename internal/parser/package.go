package parser

import "kanso/internal/ast"

func ParseSource(path string, source string) (*ast.Contract, []ParseError, []ScanError) {
	scanner := NewScanner(source)
	tokens := scanner.ScanTokens()

	parser := NewParser(path, tokens)
	contract := parser.ParseContract()

	// Assign metadata to all AST nodes
	if contract != nil {
		mv := ast.NewMetadataVisitor(source)
		// Assign metadata to leading comments
		for _, item := range contract.LeadingComments {
			mv.AssignMetadata(item, 0) // 0 = no parent
		}
		// Assign metadata to contract items
		for _, item := range contract.Items {
			mv.AssignMetadata(item, 0) // 0 = no parent
		}
	}

	return contract, parser.errors, scanner.errors
}
