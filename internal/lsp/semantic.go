package lsp

import (
	"github.com/alecthomas/participle/v2/lexer"
	"kanso/grammar"
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

func collectSemanticTokens(program *grammar.AST) []SemanticToken {
	var tokens []SemanticToken

	if program == nil {
		return tokens
	}

	for _, se := range program.SourceElements {
		if se != nil && se.Module != nil {
			tokens = append(tokens, walkModule(se.Module)...)
		}
	}

	return tokens
}

func walkModule(m *grammar.Module) []SemanticToken {
	var tokens []SemanticToken

	if m.Attribute != nil {
		tokens = append(tokens, makeToken(m.Attribute.Pos, m.Attribute.EndPos, m.Attribute.Name, "modifier", 0))
	}

	// Module name
	if m.Name.Value != "" {
		tokens = append(tokens, makeToken(m.Name.Pos, m.Name.EndPos, m.Name.Value, "namespace", 1))
	}

	// Imports (namespaces + imported names)
	for _, u := range m.Uses {
		for _, ns := range u.Namespaces {
			tokens = append(tokens, makeToken(ns.Name.Pos, ns.Name.EndPos, ns.Name.Value, "namespace", 0))
		}
		for _, imp := range u.Imports {
			tokens = append(tokens, makeToken(imp.Name.Pos, imp.Name.EndPos, imp.Name.Value, "type", 0))
		}
	}

	// Structs
	for _, s := range m.Structs {
		if s.Attribute != nil {
			tokens = append(tokens, makeToken(s.Attribute.Pos, s.Attribute.EndPos, s.Attribute.Name, "modifier", 0))
		}
		if s.Name.Value != "" {
			tokens = append(tokens, makeToken(s.Name.Pos, s.Name.EndPos, s.Name.Value, "type", 1))
		}
		for _, field := range s.Fields {
			tokens = append(tokens, makeToken(field.Name.Pos, field.Name.EndPos, field.Name.Value, "property", 1))
			tokens = append(tokens, typeReferenceToken(field.Type)...)
		}
	}

	// Functions
	for _, f := range m.Functions {
		if f.Attribute != nil {
			tokens = append(tokens, makeToken(f.Attribute.Pos, f.Attribute.EndPos, f.Attribute.Name.Value, "modifier", 0))
		}
		if f.Name.Value != "" {
			tokens = append(tokens, makeToken(f.Name.Pos, f.Name.EndPos, f.Name.Value, "function", 1))
		}

		for _, p := range f.Params {
			tokens = append(tokens, makeToken(p.Name.Pos, p.Name.EndPos, p.Name.Value, "parameter", 0))
			tokens = append(tokens, typeReferenceToken(p.Type)...)
		}
		for _, r := range f.Reads {
			tokens = append(tokens, typeReferenceToken(r)...)
		}
		for _, w := range f.Writes {
			tokens = append(tokens, typeReferenceToken(w)...)
		}
		tokens = append(tokens, walkFunctionBlock(f.Body)...)
	}

	return tokens
}

func walkFunctionBlock(fb *grammar.FunctionBlock) []SemanticToken {
	var tokens []SemanticToken

	if fb == nil {
		return tokens
	}

	for _, stmt := range fb.Statements {
		if stmt.LetStmt != nil && stmt.LetStmt.Name.Value != "" {
			tokens = append(tokens, makeToken(stmt.LetStmt.Name.Pos, stmt.LetStmt.Name.EndPos, stmt.LetStmt.Name.Value, "variable", 1))
		}
		if stmt.AssignStmt != nil && stmt.AssignStmt.Target.Value != "" {
			tokens = append(tokens, makeToken(stmt.AssignStmt.Target.Pos, stmt.AssignStmt.Target.EndPos, stmt.AssignStmt.Target.Value, "variable", 0))
		}
	}

	if fb.Tail != nil && fb.Tail.Expr != nil {
		tokens = append(tokens, walkExpr(fb.Tail.Expr)...)
	}

	return tokens
}

func walkExpr(expr *grammar.Expr) []SemanticToken {
	var tokens []SemanticToken

	if expr == nil {
		return tokens
	}

	if expr.Binary != nil {
		tokens = append(tokens, walkUnary(expr.Binary.Left)...)
		for _, op := range expr.Binary.Ops {
			tokens = append(tokens, walkUnary(op.Right)...)
		}
	}

	return tokens
}

func walkUnary(ue *grammar.UnaryExpr) []SemanticToken {
	var tokens []SemanticToken

	if ue == nil {
		return tokens
	}

	if ue.Value != nil {
		if ue.Value.Primary != nil && ue.Value.Primary.Ident != nil {
			tokens = append(tokens, makeToken(ue.Value.Primary.Ident.Pos, ue.Value.Primary.Ident.EndPos, ue.Value.Primary.Ident.Value, "variable", 0))
		}
		if ue.Value.Primary != nil && ue.Value.Primary.Call != nil {
			tokens = append(tokens, walkCallExpr(ue.Value.Primary.Call)...)
		}
	}

	return tokens
}

func walkCallExpr(call *grammar.CallExpr) []SemanticToken {
	var tokens []SemanticToken

	if call == nil {
		return tokens
	}

	// Function call like Table::empty
	for _, part := range call.Callee.Parts {
		tokens = append(tokens, makeToken(part.Pos, part.EndPos, part.Value, "function", 0))
	}

	// Generics
	for _, g := range call.Generic {
		tokens = append(tokens, typeReferenceToken(g)...)
	}

	// Args
	for _, arg := range call.Args {
		tokens = append(tokens, walkExpr(arg)...)
	}

	return tokens
}

func makeToken(pos, endPos lexer.Position, value, tokenType string, decl int) SemanticToken {
	length := endPos.Column - pos.Column
	if length <= 0 {
		length = len(value)
	}

	return SemanticToken{
		Line:           uint32(pos.Line - 1),
		StartChar:      uint32(pos.Column - 1),
		Length:         uint32(length),
		TokenType:      indexOf(tokenType, SemanticTokenTypes),
		TokenModifiers: decl << indexOf("declaration", SemanticTokenModifiers),
	}
}

// typeReferenceToken collects tokens for type references
// (e.g., parameter types, return types, generic types)
func typeReferenceToken(t *grammar.Type) []SemanticToken {
	if t == nil || t.Name.Value == "" {
		return nil
	}
	return []SemanticToken{
		makeToken(t.Name.Pos, t.Name.Pos, t.Name.Value, "type", 0),
	}
}

// indexOf returns the index of a string in a list, or -1 if not found
func indexOf(target string, list []string) int {
	for i, v := range list {
		if v == target {
			return i
		}
	}
	return -1
}
