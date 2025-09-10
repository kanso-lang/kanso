package lsp

import (
	"kanso/internal/ast"
)

// SemanticToken represents enriched syntax highlighting information for IDEs.
// This goes beyond basic syntax highlighting by providing semantic context
// (e.g., distinguishing between variable declaration vs usage).
type SemanticToken struct {
	Line           uint32
	StartChar      uint32
	Length         uint32
	TokenType      int // Maps to SemanticTokenTypes array for LSP protocol
	TokenModifiers int // Bitfield for additional styling hints
}

func collectSemanticTokens(contract *ast.Contract) []SemanticToken {
	var tokens []SemanticToken

	if contract == nil {
		return tokens
	}

	// Process comments first to ensure proper ordering in the final token list
	for _, item := range contract.LeadingComments {
		tokens = append(tokens, walkContractItem(item)...)
	}

	// Extract semantic information from all contract constructs
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
		// Documentation is handled at the tokenizer level for simplicity
		return tokens
	case *ast.Comment:
		// Basic comments don't need semantic enhancement
		return tokens
	case *ast.Struct:
		tokens = append(tokens, walkStruct(v)...)
	case *ast.Function:
		tokens = append(tokens, walkFunction(v)...)
	case *ast.Use:
		tokens = append(tokens, walkUse(v)...)
	case *ast.BadContractItem:
		// Invalid syntax should be highlighted by the basic tokenizer
		return tokens
	}

	return tokens
}

func walkUse(u *ast.Use) []SemanticToken {
	var tokens []SemanticToken

	if u == nil {
		return tokens
	}

	// Highlight module namespaces to help developers understand import hierarchy
	for _, ns := range u.Namespaces {
		tokens = append(tokens, makeToken(ns.Name.Pos, ns.Name.EndPos, ns.Name.Value, "namespace", 0)...)
	}

	// Mark imported symbols distinctly to show external dependencies
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

	// Highlight contract-specific attributes to emphasize their semantic importance
	if s.Attribute != nil {
		tokens = append(tokens, makeToken(s.Attribute.Pos, s.Attribute.EndPos, s.Attribute.Name, "modifier", 0)...)
	}

	// Mark struct names as type declarations for consistent IDE navigation
	if s.Name.Value != "" {
		tokens = append(tokens, makeToken(s.Name.Pos, s.Name.EndPos, s.Name.Value, "type", 1)...)
	}

	// Distinguish struct fields from other variables for better visual parsing
	for _, item := range s.Items {
		if field, ok := item.(*ast.StructField); ok {
			tokens = append(tokens, makeToken(field.Name.Pos, field.Name.EndPos, field.Name.Value, "property", 1)...)
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
		// Mark variable declarations to help developers distinguish from usage
		tokens = append(tokens, makeToken(v.Name.Pos, v.Name.EndPos, v.Name.Value, "variable", 1)...)
		tokens = append(tokens, walkExpression(v.Expr)...)
	case *ast.AssignStmt:
		// Process both sides of assignment for complete semantic analysis
		tokens = append(tokens, walkExpression(v.Target)...)
		tokens = append(tokens, walkExpression(v.Value)...)
	case *ast.ReturnStmt:
		if v.Value != nil {
			tokens = append(tokens, walkExpression(v.Value)...)
		}
	case *ast.ExprStmt:
		tokens = append(tokens, walkExpression(v.Expr)...)
	case *ast.Comment:
		// Delegate comment handling to basic tokenizer for consistency
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
		// Highlight variable references for IDE features like "find all references"
		tokens = append(tokens, makeToken(v.NodePos(), v.NodeEndPos(), v.Name, "variable", 0)...)
	case *ast.CallExpr:
		tokens = append(tokens, walkCallExpression(v)...)
	case *ast.FieldAccessExpr:
		// Process object access patterns for intelligent completion
		tokens = append(tokens, walkExpression(v.Target)...)
		tokens = append(tokens, makeToken(v.NodePos(), v.NodeEndPos(), v.Field, "property", 0)...)
	case *ast.StructLiteralExpr:
		// Handle struct construction syntax
		if v.Type != nil {
			tokens = append(tokens, walkCalleePath(v.Type)...)
		}
		// Analyze field initialization patterns
		for _, field := range v.Fields {
			tokens = append(tokens, makeToken(field.Name.Pos, field.Name.EndPos, field.Name.Value, "property", 0)...)
			tokens = append(tokens, walkExpression(field.Value)...)
		}
	case *ast.BinaryExpr:
		tokens = append(tokens, walkExpression(v.Left)...)
		tokens = append(tokens, walkExpression(v.Right)...)
	case *ast.UnaryExpr:
		tokens = append(tokens, walkExpression(v.Value)...)
	case *ast.ParenExpr:
		tokens = append(tokens, walkExpression(v.Value)...)
	case *ast.LiteralExpr:
		// Literal values get basic highlighting from the tokenizer
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

// makeToken converts AST position information into LSP semantic tokens.
// This bridges the gap between our internal AST representation and the LSP protocol,
// enabling rich IDE features like semantic highlighting and symbol navigation.
func makeToken(pos, endPos ast.Position, value, tokenType string, declModifier int) []SemanticToken {
	if value == "" {
		return nil
	}

	// Calculate token length, falling back to string length if position data is incomplete
	length := endPos.Column - pos.Column
	if length <= 0 {
		length = len(value)
	}

	return []SemanticToken{{
		Line:           uint32(pos.Line - 1), // Convert from 1-based to LSP's 0-based indexing
		StartChar:      uint32(pos.Column - 1),
		Length:         uint32(length),
		TokenType:      indexOf(tokenType, SemanticTokenTypes),
		TokenModifiers: declModifier << indexOf("declaration", SemanticTokenModifiers),
	}}
}

// indexOf maps string tokens to their LSP protocol indices.
// Returning 0 for unknown types ensures graceful degradation rather than crashes,
// which is important for maintaining editor stability during language evolution.
func indexOf(target string, list []string) int {
	for i, v := range list {
		if v == target {
			return i
		}
	}
	return 0 // Fallback prevents protocol violations
}
