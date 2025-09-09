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

func (p *Parser) ParseContract() *ast.Contract {
	// Collect any leading comments before the contract
	var leadingComments []ast.ContractItem
	for p.check(COMMENT) || p.check(DOC_COMMENT) {
		if p.check(DOC_COMMENT) {
			leadingComments = append(leadingComments, p.parseDocComment())
		} else {
			leadingComments = append(leadingComments, p.parseComment())
		}
	}

	// Expect 'contract' keyword
	startToken := p.consume(CONTRACT, "expected 'contract' keyword")

	// Get contract name
	nameToken := p.consume(IDENTIFIER, "expected contract name")
	contractName := ast.Ident{
		Pos:    p.makePos(nameToken),
		EndPos: p.makeEndPos(nameToken),
		Value:  nameToken.Lexeme,
	}

	// Expect opening brace
	p.consume(LEFT_BRACE, "expected '{' to start contract body")

	// Parse contract items until closing brace
	var items []ast.ContractItem
	for !p.check(RIGHT_BRACE) && !p.isAtEnd() {
		item := p.parseContractItem()
		if item != nil {
			items = append(items, item)
		} else {
			// If parsing failed, try to recover and continue
			p.synchronize()
		}
	}

	// Expect closing brace
	endToken := p.consume(RIGHT_BRACE, "expected '}' to close contract body")

	// Use the start of the first leading comment if we have any, otherwise use the contract token
	startPos := p.makePos(startToken)
	if len(leadingComments) > 0 {
		startPos = leadingComments[0].NodePos()
	}

	contract := &ast.Contract{
		Pos:             startPos,
		EndPos:          p.makeEndPos(endToken),
		LeadingComments: leadingComments,
		Name:            contractName,
		Items:           items,
	}

	return contract
}

func (p *Parser) parseContractItem() ast.ContractItem {
	// Comments must be parsed before other items to avoid consuming their tokens
	if p.peek().Type == DOC_COMMENT {
		return p.parseDocComment()
	}

	if p.peek().Type == COMMENT {
		return p.parseComment()
	}

	attr := p.parseOptionalAttribute()

	// Support doc comments after attributes since they can appear in either order
	var docComment *ast.DocComment
	if p.check(DOC_COMMENT) {
		docComment = p.parseDocComment()
	}

	// Track external modifier separately from attributes for clarity
	isExternal := false
	if p.check(EXT) {
		isExternal = true
		p.advance()
	}
	switch p.peek().Type {
	case CONTRACT:
		p.errorAtCurrent("unexpected nested contract declaration")
		bad := p.makeBadContractItem("nested contracts are not supported")
		p.advance()
		return bad
	case STRUCT:
		return p.parseStructWithDoc(attr, docComment)
	case FN:
		return p.parseFunctionWithDoc(attr, isExternal, docComment)
	case USE:
		return p.parseUse()
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
			// Currently only single attributes are supported per item
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

func (p *Parser) parseVariableType() *ast.VariableType {
	// Handle tuple types like (Address, U256)
	if p.check(LEFT_PAREN) {
		return p.parseTupleType()
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

func (p *Parser) parseTupleType() *ast.VariableType {
	start := p.consume(LEFT_PAREN, "expected '(' for tuple type")

	typ := &ast.VariableType{
		Pos: p.makePos(start),
	}

	if !p.check(RIGHT_PAREN) {
		for {
			element := p.parseVariableType()
			typ.TupleElements = append(typ.TupleElements, element)
			if !p.match(COMMA) {
				break
			}
		}
	}

	end := p.consume(RIGHT_PAREN, "expected ')' to close tuple type")
	typ.EndPos = p.makeEndPos(end)

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
