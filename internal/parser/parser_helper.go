package parser

import "kanso/internal/ast"

func (p *Parser) advance() Token {
	if !p.isAtEnd() {
		p.current++
	}
	return p.previous()
}

func (p *Parser) check(tt TokenType) bool {
	if p.isAtEnd() {
		return false
	}
	return p.peek().Type == tt
}

func (p *Parser) match(types ...TokenType) bool {
	for _, tt := range types {
		if p.check(tt) {
			p.advance()
			return true
		}
	}
	return false
}

func (p *Parser) consume(tt TokenType, message string) Token {
	if p.check(tt) {
		return p.advance()
	}
	p.errorAtCurrent(message)
	illegal := Token{Type: ILLEGAL, Position: p.peek().Position}
	p.advance()
	return illegal
}

func (p *Parser) peek() Token {
	return p.tokens[p.current]
}

func (p *Parser) previous() Token {
	return p.tokens[p.current-1]
}

func (p *Parser) isAtEnd() bool {
	return p.peek().Type == EOF
}

func (p *Parser) errorAtCurrent(message string) {
	pos := p.peek().Position
	p.errors = append(p.errors, ParseError{
		Message:  message,
		Position: pos,
	})
}

func (p *Parser) makePos(tok Token) ast.Position {
	return ast.Position{
		Filename: p.filename, // assuming Parser has a `filename` field
		Offset:   tok.Position.Offset,
		Line:     tok.Position.Line,
		Column:   tok.Position.Column,
	}
}

func (p *Parser) makeEndPos(tok Token) ast.Position {
	return ast.Position{
		Filename: p.filename,
		Offset:   tok.Position.Offset + len(tok.Lexeme),
		Line:     tok.Position.Line,
		Column:   tok.Position.Column + len(tok.Lexeme),
	}
}

func (p *Parser) synchronize() {
	p.advance()

	for !p.isAtEnd() {
		if p.previous().Type == SEMICOLON {
			return
		}

		switch p.peek().Type {
		case FUN, LET, IF, RETURN, MODULE:
			return
		}

		p.advance()
	}
}

// Helper functions to reduce repetitive AST node creation

// makeIdent creates an ast.Ident from a token
func (p *Parser) makeIdent(tok Token) ast.Ident {
	return ast.Ident{
		Pos:    p.makePos(tok),
		EndPos: p.makeEndPos(tok),
		Value:  tok.Lexeme,
	}
}

// consumeIdent consumes an identifier token and returns an ast.Ident
func (p *Parser) consumeIdent(message string) (ast.Ident, bool) {
	tok := p.consume(IDENTIFIER, message)
	if tok.Type == ILLEGAL {
		return ast.Ident{Value: "error"}, false
	}
	return p.makeIdent(tok), true
}

// parseIdentifierList parses a comma-separated list of identifiers
func (p *Parser) parseIdentifierList() []ast.Ident {
	var idents []ast.Ident

	for !p.isAtEnd() {
		ident, ok := p.consumeIdent("expected identifier")
		if !ok {
			break
		}
		idents = append(idents, ident)

		if !p.match(COMMA) {
			break
		}
	}

	return idents
}

// parseOptionalParenIdentifierList parses optional parenthesized identifier list
// e.g., reads(State) or writes(State, Account)
func (p *Parser) parseOptionalParenIdentifierList() []ast.Ident {
	var idents []ast.Ident

	if p.match(LEFT_PAREN) {
		idents = p.parseIdentifierList()
		p.consume(RIGHT_PAREN, "expected ')' to close identifier list")
	} else {
		// Single identifier without parentheses
		ident, ok := p.consumeIdent("expected identifier")
		if ok {
			idents = append(idents, ident)
		}
	}

	return idents
}
