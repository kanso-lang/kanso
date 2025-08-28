package parser

import "kanso/internal/ast"

func ParseSource(path string, source string) (*ast.Contract, []ParseError, []ScanError) {
	scanner := NewScanner(source)
	tokens := scanner.ScanTokens()

	parser := NewParser(path, tokens)
	contract := parser.ParseContract()

	return contract, parser.errors, scanner.errors
}
