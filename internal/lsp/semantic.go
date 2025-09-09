package lsp

import (
	"kanso/internal/ast"
)

// SemanticToken represents a single LSP semantic token entry
// Line and StartChar are 0-based positions
// TokenType is an index into the semanticTokenTypes array
// TokenModifiers is a bitmask based on semanticTokenModifiers
type SemanticToken struct {
	Line           uint32
	StartChar      uint32
	Length         uint32
	TokenType      int // index into semanticTokenTypes
	TokenModifiers int // bitmask
}

func collectSemanticTokens(contract *ast.Contract) []SemanticToken {
	var tokens []SemanticToken

	if contract == nil {
		return tokens
	}

	// Walk through leading comments first
	for _, item := range contract.LeadingComments {
		tokens = append(tokens, walkContractItem(item)...)
	}

	// Walk through all contract items
	for _, item := range contract.Items {
		tokens = append(tokens, walkContractItem(item)...)
	}

	return tokens
}

func walkContractItem(item ast.ContractItem) []SemanticToken {
	var tokens []SemanticToken

	if item == nil {
		return tokens
	}

	switch v := item.(type) {
	case *ast.DocComment:
		// Doc comments are already handled by the tokenizer
		return tokens
	case *ast.Comment:
		// Regular comments are already handled by the tokenizer
		return tokens
	case *ast.Struct:
		tokens = append(tokens, walkStruct(v)...)
	case *ast.Function:
		tokens = append(tokens, walkFunction(v)...)
	case *ast.Use:
		tokens = append(tokens, walkUse(v)...)
	case *ast.BadContractItem:
		// Skip bad items
		return tokens
	}

	return tokens
}

func walkUse(u *ast.Use) []SemanticToken {
	var tokens []SemanticToken

	if u == nil {
		return tokens
	}

	// Namespaces (like "Evm", "Table")
	for _, ns := range u.Namespaces {
		tokens = append(tokens, makeToken(ns.Name.Pos, ns.Name.EndPos, ns.Name.Value, "namespace", 0)...)
	}

	// Imported items (like "sender", "emit")
	for _, imp := range u.Imports {
		tokens = append(tokens, makeToken(imp.Name.Pos, imp.Name.EndPos, imp.Name.Value, "type", 0)...)
	}

	return tokens
}

func walkStruct(s *ast.Struct) []SemanticToken {
	var tokens []SemanticToken

	if s == nil {
		return tokens
	}

	// Struct attribute (like #[storage], #[event])
	if s.Attribute != nil {
		tokens = append(tokens, makeToken(s.Attribute.Pos, s.Attribute.EndPos, s.Attribute.Name, "modifier", 0)...)
	}

	// Struct name
	if s.Name.Value != "" {
		tokens = append(tokens, makeToken(s.Name.Pos, s.Name.EndPos, s.Name.Value, "type", 1)...)
	}

	// Struct fields
	for _, item := range s.Items {
		if field, ok := item.(*ast.StructField); ok {
			// Field name
			tokens = append(tokens, makeToken(field.Name.Pos, field.Name.EndPos, field.Name.Value, "property", 1)...)
			// Field type
			tokens = append(tokens, walkVariableType(field.VariableType)...)
		}
	}

	return tokens
}

func walkFunction(f *ast.Function) []SemanticToken {
	var tokens []SemanticToken

	if f == nil {
		return tokens
	}

	// Function attribute (like #[create])
	if f.Attribute != nil {
		tokens = append(tokens, makeToken(f.Attribute.Pos, f.Attribute.EndPos, f.Attribute.Name, "modifier", 0)...)
	}

	// Function name
	if f.Name.Value != "" {
		tokens = append(tokens, makeToken(f.Name.Pos, f.Name.EndPos, f.Name.Value, "function", 1)...)
	}

	// Parameters
	for _, param := range f.Params {
		if param != nil {
			// Parameter name
			tokens = append(tokens, makeToken(param.Name.Pos, param.Name.EndPos, param.Name.Value, "parameter", 0)...)
			// Parameter type
			tokens = append(tokens, walkVariableType(param.Type)...)
		}
	}

	// Return type
	if f.Return != nil {
		tokens = append(tokens, walkVariableType(f.Return)...)
	}

	// Reads clause
	for _, read := range f.Reads {
		tokens = append(tokens, makeToken(read.Pos, read.EndPos, read.Value, "type", 0)...)
	}

	// Writes clause
	for _, write := range f.Writes {
		tokens = append(tokens, makeToken(write.Pos, write.EndPos, write.Value, "type", 0)...)
	}

	// Function body
	if f.Body != nil {
		tokens = append(tokens, walkFunctionBlock(f.Body)...)
	}

	return tokens
}

func walkFunctionBlock(fb *ast.FunctionBlock) []SemanticToken {
	var tokens []SemanticToken

	if fb == nil {
		return tokens
	}

	// Function body items
	for _, item := range fb.Items {
		tokens = append(tokens, walkFunctionBlockItem(item)...)
	}

	// Tail expression
	if fb.TailExpr != nil {
		tokens = append(tokens, walkExpression(fb.TailExpr.Expr)...)
	}

	return tokens
}

func walkFunctionBlockItem(item ast.FunctionBlockItem) []SemanticToken {
	var tokens []SemanticToken

	if item == nil {
		return tokens
	}

	switch v := item.(type) {
	case *ast.LetStmt:
		// Variable name
		tokens = append(tokens, makeToken(v.Name.Pos, v.Name.EndPos, v.Name.Value, "variable", 1)...)
		// Variable value expression
		tokens = append(tokens, walkExpression(v.Expr)...)
	case *ast.AssignStmt:
		// Assignment target
		tokens = append(tokens, walkExpression(v.Target)...)
		// Assignment value
		tokens = append(tokens, walkExpression(v.Value)...)
	case *ast.ReturnStmt:
		// Return value
		if v.Value != nil {
			tokens = append(tokens, walkExpression(v.Value)...)
		}
	case *ast.ExprStmt:
		// Expression statement
		tokens = append(tokens, walkExpression(v.Expr)...)
	case *ast.Comment:
		// Comments are handled by tokenizer
		return tokens
	}

	return tokens
}

func walkExpression(expr ast.Expr) []SemanticToken {
	var tokens []SemanticToken

	if expr == nil {
		return tokens
	}

	switch v := expr.(type) {
	case *ast.IdentExpr:
		// Variable references
		tokens = append(tokens, makeToken(v.NodePos(), v.NodeEndPos(), v.Name, "variable", 0)...)
	case *ast.CallExpr:
		tokens = append(tokens, walkCallExpression(v)...)
	case *ast.FieldAccessExpr:
		// Object being accessed
		tokens = append(tokens, walkExpression(v.Target)...)
		// Field name - treat as property access
		tokens = append(tokens, makeToken(v.NodePos(), v.NodeEndPos(), v.Field, "property", 0)...)
	case *ast.StructLiteralExpr:
		// Struct type - v.Type is a *CalleePath, not *VariableType
		if v.Type != nil {
			tokens = append(tokens, walkCalleePath(v.Type)...)
		}
		// Struct fields
		for _, field := range v.Fields {
			// Field name
			tokens = append(tokens, makeToken(field.Name.Pos, field.Name.EndPos, field.Name.Value, "property", 0)...)
			// Field value
			tokens = append(tokens, walkExpression(field.Value)...)
		}
	case *ast.BinaryExpr:
		// Left and right expressions
		tokens = append(tokens, walkExpression(v.Left)...)
		tokens = append(tokens, walkExpression(v.Right)...)
	case *ast.UnaryExpr:
		// Unary expression value
		tokens = append(tokens, walkExpression(v.Value)...)
	case *ast.ParenExpr:
		// Parenthesized expression
		tokens = append(tokens, walkExpression(v.Value)...)
	case *ast.LiteralExpr:
		// Literals don't need special semantic tokens
		return tokens
	}

	return tokens
}

func walkCallExpression(call *ast.CallExpr) []SemanticToken {
	var tokens []SemanticToken

	if call == nil {
		return tokens
	}

	// Function being called
	tokens = append(tokens, walkExpression(call.Callee)...)

	// Generic type arguments
	for _, generic := range call.Generic {
		tokens = append(tokens, walkVariableType(&generic)...)
	}

	// Function arguments
	for _, arg := range call.Args {
		tokens = append(tokens, walkExpression(arg)...)
	}

	return tokens
}

func walkVariableType(vt *ast.VariableType) []SemanticToken {
	var tokens []SemanticToken

	if vt == nil {
		return tokens
	}

	// Type name
	if vt.Name.Value != "" {
		tokens = append(tokens, makeToken(vt.Name.Pos, vt.Name.EndPos, vt.Name.Value, "type", 0)...)
	}

	// Generic parameters
	for _, generic := range vt.Generics {
		tokens = append(tokens, walkVariableType(generic)...)
	}

	return tokens
}

func walkCalleePath(cp *ast.CalleePath) []SemanticToken {
	var tokens []SemanticToken

	if cp == nil {
		return tokens
	}

	// Walk through each part of the callee path (e.g., Table::empty)
	for _, part := range cp.Parts {
		tokens = append(tokens, makeToken(part.Pos, part.EndPos, part.Value, "function", 0)...)
	}

	return tokens
}

// makeToken creates a semantic token for a given position and text
func makeToken(pos, endPos ast.Position, value, tokenType string, declModifier int) []SemanticToken {
	if value == "" {
		return nil
	}

	length := endPos.Column - pos.Column
	if length <= 0 {
		length = len(value)
	}

	return []SemanticToken{{
		Line:           uint32(pos.Line - 1),   // LSP uses 0-based line numbers
		StartChar:      uint32(pos.Column - 1), // LSP uses 0-based column numbers
		Length:         uint32(length),
		TokenType:      indexOf(tokenType, SemanticTokenTypes),
		TokenModifiers: declModifier << indexOf("declaration", SemanticTokenModifiers),
	}}
}

// indexOf returns the index of a string in a slice, or 0 if not found
func indexOf(target string, list []string) int {
	for i, v := range list {
		if v == target {
			return i
		}
	}
	return 0 // Default to first token type if not found
}
