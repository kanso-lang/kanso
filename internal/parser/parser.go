package parser

import (
	"fmt"
	"kanso/internal/ast"
)

type Parser struct {
	tokens   []Token
	filename string
	current  int
	errors   []ParseError
}

type ParseError struct {
	Message  string
	Position Position
}

func NewParser(filename string, tokens []Token) *Parser {
	return &Parser{
		tokens:   tokens,
		filename: filename,
	}
}

// ParseContract is the main entry point for parsing a Kanso source file.
// It parses the entire contract and returns the AST along with any errors.
func (p *Parser) ParseContract() *ast.Contract {
	contract := &ast.Contract{}

	// Parse all top-level items until end of file
	for !p.isAtEnd() {
		item := p.parseContractItem()
		if item != nil {
			contract.ContractItems = append(contract.ContractItems, item)
		} else {
			// If parsing failed, try to recover and continue
			p.synchronize()
		}
	}

	return contract
}

// parseContractItem parses a single top-level item in a contract file.
// This includes comments, modules with optional attributes.
func (p *Parser) parseContractItem() ast.ContractItem {
	// Handle doc comments and regular comments first
	if p.peek().Type == DOC_COMMENT {
		return p.parseDocComment()
	}

	if p.peek().Type == COMMENT {
		return p.parseComment()
	}

	// Parse optional attribute (like #[contract])
	attr := p.parseOptionalAttribute()

	// Parse the actual item
	switch p.peek().Type {
	case MODULE:
		return p.parseModule(attr)
	default:
		p.errorAtCurrent("unexpected top-level item")
		bad := p.makeBadContractItem("unexpected token at contract level")
		p.advance()
		return bad
	}
}

func (p *Parser) parseDocComment() *ast.DocComment {
	tok := p.advance()
	return &ast.DocComment{
		Pos:    p.makePos(tok),
		EndPos: p.makeEndPos(tok),
		Text:   tok.Lexeme,
	}
}

func (p *Parser) parseComment() *ast.Comment {
	tok := p.advance()
	return &ast.Comment{
		Pos:    p.makePos(tok),
		EndPos: p.makeEndPos(tok),
		Text:   tok.Lexeme,
	}
}

func (p *Parser) parseOptionalAttribute() *ast.Attribute {
	var attr *ast.Attribute = nil
	if p.peek().Type == POUND {
		attr = p.parseAttribute()

		if p.peek().Type == POUND {
			p.errorAtCurrent("multiple attributes are not supported")
			// You can decide whether to:
			// - parse and discard the extra attribute(s)
			// - skip forward
			// For now, just parse one and leave the rest unparsed.
		}
	}
	return attr
}

func (p *Parser) parseAttribute() *ast.Attribute {
	if !p.match(POUND) {
		return nil
	}

	if !p.match(LEFT_BRACKET) {
		p.errorAtCurrent("expected '[' after '#' for attribute")
		return nil
	}

	nameToken := p.consume(IDENTIFIER, "expected identifier inside attribute")

	if !p.match(RIGHT_BRACKET) {
		p.errorAtCurrent("expected closing ']' after attribute")
		return nil
	}

	return &ast.Attribute{
		Pos:    p.makePos(nameToken),
		EndPos: p.makeEndPos(nameToken),
		Name:   nameToken.Lexeme,
	}
}

func (p *Parser) parseModule(attr *ast.Attribute) *ast.Module {
	mod := &ast.Module{
		Pos:        p.makePos(p.peek()),
		Attributes: []ast.Attribute{},
	}

	p.consume(MODULE, "expected 'module' keyword")
	nameToken := p.consume(IDENTIFIER, "expected module name")
	mod.Name = ast.Ident{
		Pos:    p.makePos(nameToken),
		EndPos: p.makeEndPos(nameToken),
		Value:  nameToken.Lexeme,
	}

	p.consume(LEFT_BRACE, "expected '{' to start module body")

	for !p.check(RIGHT_BRACE) && !p.isAtEnd() {
		item := p.parseModuleItem()
		if item != nil {
			mod.ModuleItems = append(mod.ModuleItems, item)
		} else {
			p.synchronize()
		}
	}

	mod.EndPos = p.makeEndPos(p.consume(RIGHT_BRACE, "expected '}' to close module body"))

	if attr != nil {
		mod.Attributes = append(mod.Attributes, *attr)
	}

	return mod
}

func (p *Parser) parseModuleItem() ast.ModuleItem {
	attr := p.parseOptionalAttribute()

	if p.peek().Type == DOC_COMMENT {
		return p.parseDocComment()
	}

	if p.peek().Type == COMMENT {
		return p.parseComment()
	}

	isPublic := false
	if p.check(PUBLIC) {
		isPublic = true
		p.advance()
	}

	switch p.peek().Type {
	case STRUCT:
		return p.parseStruct(attr)
	case FUN:
		return p.parseFunction(attr, isPublic)
	case USE:
		return p.parseUse()
	default:
		p.errorAtCurrent("unexpected item in module body")

		bmi := &ast.BadModuleItem{
			Bad: ast.BadNode{
				Pos:     p.makePos(p.peek()),
				EndPos:  p.makeEndPos(p.peek()),
				Message: "unexpected token in module body",
				Details: []string{p.peek().Lexeme},
			},
		}

		p.advance()
		return bmi
	}
}

func (p *Parser) parseVariableType() *ast.VariableType {
	start := p.peek()
	var ref *ast.RefVariableType

	if p.match(AMPERSAND) {
		mut := p.match(MUT)
		target := p.parseVariableType()
		ref = &ast.RefVariableType{
			Pos:    p.makePos(start),
			EndPos: target.EndPos,
			And:    true,
			Mut:    mut,
			Target: target,
		}
		return &ast.VariableType{
			Pos:    p.makePos(start),
			EndPos: target.EndPos,
			Ref:    ref,
		}
	}

	nameTok := p.consume(IDENTIFIER, "expected type name")

	typ := &ast.VariableType{
		Pos: p.makePos(nameTok),
		Name: ast.Ident{
			Pos:    p.makePos(nameTok),
			EndPos: p.makeEndPos(nameTok),
			Value:  nameTok.Lexeme,
		},
	}

	if p.match(LESS) {
		for {
			sub := p.parseVariableType()
			typ.Generics = append(typ.Generics, sub)
			if !p.match(COMMA) {
				break
			}
		}
		end := p.consume(GREATER, "expected '>' to close generics")
		typ.EndPos = p.makeEndPos(end)
	} else {
		typ.EndPos = typ.Name.EndPos
	}

	return typ
}

func (p *Parser) makeBadContractItem(message string) *ast.BadContractItem {
	tok := p.peek()
	badNode := ast.BadNode{
		Pos:     p.makePos(tok),
		EndPos:  p.makeEndPos(tok),
		Message: message,
		Details: []string{fmt.Sprintf("unexpected token: %s", tok.Lexeme)},
	}
	return &ast.BadContractItem{Bad: badNode}
}

func (p *Parser) makeBadExpr(message string) *ast.BadExpr {
	tok := p.peek()
	badNode := ast.BadNode{
		Pos:     p.makePos(tok),
		EndPos:  p.makeEndPos(tok),
		Message: message,
		Details: []string{fmt.Sprintf("unexpected expr: %s", tok.Lexeme)},
	}
	return &ast.BadExpr{Bad: badNode}
}
