package parser

import (
	"kanso/internal/ast"
)

var binaryPrecedence = map[string]int{
	"||": 1,
	"&&": 2,
	"==": 3, "!=": 3,
	"<": 4, "<=": 4, ">": 4, ">=": 4,
	"+": 5, "-": 5,
	"*": 6, "/": 6, "%": 6,
}

func (p *Parser) parsePrattExpr(minPrec int) ast.Expr {
	expr := p.parsePrefixExpr()

	for {
		tok := p.peek()
		prec, ok := binaryPrecedence[tok.Lexeme]
		if !ok || prec < minPrec {
			break
		}

		p.advance()
		right := p.parsePrattExpr(prec + 1)

		expr = &ast.BinaryExpr{
			Pos:    expr.NodePos(),
			EndPos: right.NodeEndPos(),
			Op:     tok.Lexeme,
			Left:   expr,
			Right:  right,
		}
	}

	return p.parsePostfixExpr(expr)
}

func (p *Parser) parsePrefixExpr() ast.Expr {
	if p.match(AMPERSAND) {
		mut := false
		if p.match(MUT) {
			mut = true
		}

		value := p.parsePrefixExpr()
		return &ast.UnaryExpr{
			Pos:    p.makePos(p.previous()), // '&' or 'mut'
			EndPos: value.NodeEndPos(),
			Op:     "&",
			Value:  value,
			Mut:    mut,
		}
	}

	if p.match(MINUS, BANG, STAR) {
		op := p.previous()
		value := p.parsePrefixExpr()
		return &ast.UnaryExpr{
			Pos:    p.makePos(op),
			EndPos: value.NodeEndPos(),
			Op:     op.Lexeme,
			Value:  value,
		}
	}

	return p.parsePrimaryExpr()
}

func (p *Parser) parsePostfixExpr(expr ast.Expr) ast.Expr {
	for {
		if p.match(DOT) {
			field := p.consume(IDENTIFIER, "expected field name after '.'")
			expr = &ast.FieldAccessExpr{
				Pos:    expr.NodePos(),
				EndPos: p.makeEndPos(field),
				Target: expr,
				Field:  field.Lexeme,
			}
		} else if p.check(LEFT_PAREN) {
			p.advance()
			args := p.parseExprList()
			end := p.consume(RIGHT_PAREN, "expected ')' after arguments")
			expr = &ast.CallExpr{
				Pos:    expr.NodePos(),
				EndPos: p.makeEndPos(end),
				Callee: expr,
				Args:   args,
			}
		} else if p.check(LEFT_BRACKET) {
			p.advance()
			index := p.parseExpr()
			end := p.consume(RIGHT_BRACKET, "expected ']' after index")
			expr = &ast.IndexExpr{
				Pos:    expr.NodePos(),
				EndPos: p.makeEndPos(end),
				Target: expr,
				Index:  index,
			}
		} else {
			break
		}
	}

	return expr
}

func (p *Parser) parsePrimaryExpr() ast.Expr {
	if p.match(NUMBER, HEX_NUMBER, STRING) {
		tok := p.previous()
		return &ast.LiteralExpr{
			Pos:    p.makePos(tok),
			EndPos: p.makeEndPos(tok),
			Value:  tok.Lexeme,
		}
	}

	if p.match(IDENTIFIER) {
		start := p.previous()
		parts := []ast.Ident{{
			Pos:    p.makePos(start),
			EndPos: p.makeEndPos(start),
			Value:  start.Lexeme,
		}}

		for p.match(DOUBLE_COLON) {
			next := p.consume(IDENTIFIER, "expected identifier after '::'")
			parts = append(parts, ast.Ident{
				Pos:    p.makePos(next),
				EndPos: p.makeEndPos(next),
				Value:  next.Lexeme,
			})
		}

		path := &ast.CalleePath{
			Pos:    parts[0].Pos,
			EndPos: parts[len(parts)-1].EndPos,
			Parts:  parts,
		}

		var genericArgs []ast.VariableType
		if p.match(LESS) {
			genericArgs = p.parseGenericTypeArgs()
		}

		if p.check(LEFT_PAREN) {
			p.advance()
			args := p.parseExprList()
			rparen := p.consume(RIGHT_PAREN, "expected ')' after arguments")
			return &ast.CallExpr{
				Pos:     path.Pos,
				EndPos:  p.makeEndPos(rparen),
				Callee:  path,
				Generic: genericArgs,
				Args:    args,
			}
		}

		if p.check(LEFT_BRACE) {
			p.advance()
			return p.parseStructLiteralExpr(path)
		}

		// Optimize for single identifiers to avoid unnecessary CalleePath wrapper
		if len(path.Parts) == 1 {
			return &ast.IdentExpr{
				Pos:    path.Pos,
				EndPos: path.EndPos,
				Name:   path.Parts[0].Value,
			}
		}

		return path
	}

	if p.match(LEFT_PAREN) {
		l := p.previous()

		if p.check(RIGHT_PAREN) {
			r := p.advance()
			return &ast.TupleExpr{
				Pos:      p.makePos(l),
				EndPos:   p.makeEndPos(r),
				Elements: []ast.Expr{},
			}
		}

		first := p.parsePrattExpr(0)

		// Distinguish between tuple (a, b) and parenthesized expression (a)
		if p.match(COMMA) {
			elements := []ast.Expr{first}

			// Continue parsing until we hit the closing paren or error
			if !p.check(RIGHT_PAREN) {
				for {
					elem := p.parsePrattExpr(0)
					elements = append(elements, elem)
					if !p.match(COMMA) {
						break
					}
					// Support trailing commas like Rust/Go
					if p.check(RIGHT_PAREN) {
						break
					}
				}
			}

			r := p.consume(RIGHT_PAREN, "expected ')' after tuple elements")
			return &ast.TupleExpr{
				Pos:      p.makePos(l),
				EndPos:   p.makeEndPos(r),
				Elements: elements,
			}
		}

		r := p.consume(RIGHT_PAREN, "expected ')'")
		return &ast.ParenExpr{
			Pos:    p.makePos(l),
			EndPos: p.makeEndPos(r),
			Value:  first,
		}
	}

	tok := p.peek()
	p.errorAtCurrent("unexpected token in expression")
	bad := &ast.BadExpr{
		Bad: ast.BadNode{
			Pos:     p.makePos(tok),
			EndPos:  p.makeEndPos(tok),
			Message: "unexpected token in expression: " + tok.Lexeme,
		},
	}
	p.advance()
	return bad
}

func (p *Parser) parseExprList() []ast.Expr {
	var args []ast.Expr
	if p.check(RIGHT_PAREN) {
		return args
	}

	for {
		args = append(args, p.parsePrattExpr(0))
		if !p.match(COMMA) {
			break
		}
	}

	return args
}

func (p *Parser) parseGenericTypeArgs() []ast.VariableType {
	var types []ast.VariableType

	// Parse first type
	if !p.check(GREATER) { // Don't parse if immediately >
		ty := p.parseType()
		types = append(types, *ty)

		// Parse remaining types separated by commas
		for p.match(COMMA) {
			ty := p.parseType()
			types = append(types, *ty)
		}
	}

	// Consume closing >
	p.consume(GREATER, "expected '>' after generic arguments")
	return types
}

func (p *Parser) parseType() *ast.VariableType {
	if !p.match(IDENTIFIER) {
		tok := p.peek()
		p.errorAtCurrent("expected type identifier")
		bad := &ast.VariableType{
			Pos:    p.makePos(tok),
			EndPos: p.makeEndPos(tok),
			Name:   ast.Ident{Value: "error"},
		}
		p.advance()
		return bad
	}

	id := p.previous()
	name := ast.Ident{
		Pos:    p.makePos(id),
		EndPos: p.makeEndPos(id),
		Value:  id.Lexeme,
	}

	var generics []*ast.VariableType
	if p.match(LESS) {
		// Parse first generic parameter
		if !p.check(GREATER) {
			generics = append(generics, p.parseType())

			// Parse remaining parameters separated by commas
			for p.match(COMMA) {
				generics = append(generics, p.parseType())
			}
		}

		// Consume closing >
		closingPos := p.consume(GREATER, "expected '>' after generic parameters")
		name.EndPos = p.makeEndPos(closingPos)
	}

	return &ast.VariableType{
		Pos:      name.Pos,
		EndPos:   name.EndPos,
		Name:     name,
		Generics: generics,
	}
}

func (p *Parser) parseStructLiteralExpr(path *ast.CalleePath) ast.Expr {
	start := p.previous() // should be LEFT_BRACE
	var fields []ast.StructLiteralField

	for !p.check(RIGHT_BRACE) && !p.isAtEnd() {
		if !p.check(IDENTIFIER) {
			tok := p.peek()
			p.errorAtCurrent("expected field name")
			bad := ast.StructLiteralField{
				Pos:    p.makePos(start),
				EndPos: p.makeEndPos(tok),
				Name:   ast.Ident{Value: "error"},
				Value: &ast.BadExpr{
					Bad: ast.BadNode{
						Pos:     p.makePos(tok),
						EndPos:  p.makeEndPos(tok),
						Message: "invalid field name",
					},
				},
			}
			fields = append(fields, bad)
			p.synchronizeUntil(COMMA, RIGHT_BRACE)
			if p.match(COMMA) {
				continue
			} else {
				break
			}
		}

		nameTok := p.advance()
		nameIdent := ast.Ident{
			Pos:    p.makePos(nameTok),
			EndPos: p.makeEndPos(nameTok),
			Value:  nameTok.Lexeme,
		}

		// Support shorthand syntax: `name,`
		if !p.match(COLON) {
			fields = append(fields, ast.StructLiteralField{
				Pos:    nameIdent.Pos,
				EndPos: nameIdent.EndPos,
				Name:   nameIdent,
				Value: &ast.IdentExpr{
					Pos:    nameIdent.Pos,
					EndPos: nameIdent.EndPos,
					Name:   nameIdent.Value,
				},
			})
			if !p.match(COMMA) {
				break
			}
			continue
		}

		expr := p.parsePrattExpr(0)
		fields = append(fields, ast.StructLiteralField{
			Pos:    nameIdent.Pos,
			EndPos: expr.NodeEndPos(),
			Name:   nameIdent,
			Value:  expr,
		})

		if !p.match(COMMA) {
			break
		}
	}

	end := p.consume(RIGHT_BRACE, "expected '}' after struct literal")
	name := path.Parts[len(path.Parts)-1].Value

	return &ast.StructLiteralExpr{
		Pos:    path.Pos,
		EndPos: p.makeEndPos(end),
		Name:   name,
		Type:   path,
		Fields: fields,
	}
}

func (p *Parser) synchronizeUntil(stopTokens ...TokenType) {
	stop := make(map[TokenType]struct{})
	for _, t := range stopTokens {
		stop[t] = struct{}{}
	}

	for !p.isAtEnd() {
		if _, ok := stop[p.peek().Type]; ok {
			return
		}
		p.advance()
	}
}
