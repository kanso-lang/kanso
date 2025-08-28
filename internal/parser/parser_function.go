package parser

import "kanso/internal/ast"

func (p *Parser) parseFunction(attr *ast.Attribute, isPublic bool) *ast.Function {
	startToken := p.consume(FUN, "expected 'fun' keyword")

	// Parse function name
	name, ok := p.consumeIdent("expected function name")
	if !ok {
		p.synchronize()
		return nil
	}

	// Parse parameters
	params := p.parseFunctionParameters()

	// Parse optional return type
	returnType := p.parseFunctionReturnType()

	// Parse optional reads clause
	reads := p.parseFunctionReadsClause()

	// Parse optional writes clause
	writes := p.parseFunctionWritesClause()

	// Parse function body
	body := p.parseFunctionBlock()
	if body.Pos == (ast.Position{}) { // recovery failed
		p.synchronize()
		return nil
	}

	return &ast.Function{
		Pos:       p.makePos(startToken),
		EndPos:    body.EndPos,
		Attribute: attr,
		Public:    isPublic,
		Name:      name,
		Params:    params,
		Return:    returnType,
		Reads:     reads,
		Writes:    writes,
		Body:      &body,
	}
}

// parseFunctionParameters parses the parameter list in parentheses
func (p *Parser) parseFunctionParameters() []*ast.FunctionParam {
	p.consume(LEFT_PAREN, "expected '(' after function name")
	var params []*ast.FunctionParam

	for !p.check(RIGHT_PAREN) && !p.isAtEnd() {
		paramName, ok := p.consumeIdent("expected parameter name")
		if !ok {
			break
		}

		p.consume(COLON, "expected ':' after parameter name")
		paramType := p.parseVariableType()

		params = append(params, &ast.FunctionParam{
			Name: paramName,
			Type: paramType,
		})

		if !p.match(COMMA) {
			break
		}
	}

	p.consume(RIGHT_PAREN, "expected ')' after parameter list")
	return params
}

// parseFunctionReturnType parses the optional return type after ':'
func (p *Parser) parseFunctionReturnType() *ast.VariableType {
	if p.match(COLON) {
		return p.parseVariableType()
	}
	return nil
}

// parseFunctionReadsClause parses the optional 'reads(...)' clause
func (p *Parser) parseFunctionReadsClause() []ast.Ident {
	if p.match(READS) {
		return p.parseOptionalParenIdentifierList()
	}
	return nil
}

// parseFunctionWritesClause parses the optional 'writes(...)' clause
func (p *Parser) parseFunctionWritesClause() []ast.Ident {
	if p.match(WRITES) {
		return p.parseOptionalParenIdentifierList()
	}
	return nil
}

func (p *Parser) parseFunctionBlock() ast.FunctionBlock {
	start := p.consume(LEFT_BRACE, "expected '{' to start function body")
	var items []ast.FunctionBlockItem
	var tail *ast.ExprStmt

	for !p.check(RIGHT_BRACE) && !p.isAtEnd() {
		if p.check(RETURN) {
			stmt := p.parseReturnStmt()
			items = append(items, stmt)
		} else if p.check(LET) {
			stmt := p.parseLetStmt()
			items = append(items, stmt)
		} else if p.check(ASSERT) {
			stmt := p.parseAssertStmt()
			items = append(items, stmt)
		} else if p.check(COMMENT) {
			token := p.advance()
			items = append(items, &ast.Comment{
				Pos:    p.makePos(token),
				EndPos: p.makeEndPos(token),
				Text:   token.Lexeme,
			})
		} else {
			expr := p.parseExpr()

			if _, bad := expr.(*ast.BadExpr); bad {
				p.synchronize()
				continue
			}

			if isAssignable(expr) && isAssignOperator(p.peek()) {
				opTok := p.advance()
				value := p.parseExpr()
				semi := p.consume(SEMICOLON, "expected ';' after assignment")

				items = append(items, &ast.AssignStmt{
					Pos:      expr.NodePos(),
					EndPos:   p.makeEndPos(semi),
					Target:   expr,
					Operator: assignOpFromToken(opTok),
					Value:    value,
				})
				continue
			}

			if p.match(SEMICOLON) {
				items = append(items, &ast.ExprStmt{
					Pos:       expr.NodePos(),
					EndPos:    p.makeEndPos(p.previous()),
					Expr:      expr,
					Semicolon: true,
				})
			} else if p.check(RIGHT_BRACE) {
				tail = &ast.ExprStmt{
					Pos:       expr.NodePos(),
					EndPos:    expr.NodeEndPos(),
					Expr:      expr,
					Semicolon: false,
				}
				break
			} else {
				semi := p.consume(SEMICOLON, "expected ';' or '}' after expression")
				items = append(items, &ast.ExprStmt{
					Pos:       expr.NodePos(),
					EndPos:    p.makeEndPos(semi),
					Expr:      expr,
					Semicolon: true,
				})
			}
		}
	}

	end := p.consume(RIGHT_BRACE, "expected '}' to close function body")
	return ast.FunctionBlock{
		Pos:      p.makePos(start),
		EndPos:   p.makeEndPos(end),
		Items:    items,
		TailExpr: tail,
	}
}

func (p *Parser) parseLetStmt() *ast.LetStmt {
	start := p.consume(LET, "expected 'let'")
	name, ok := p.consumeIdent("expected variable name after 'let'")
	if !ok {
		return nil
	}

	p.consume(EQUAL, "expected '=' in let statement")
	expr := p.parseExpr()
	semi := p.consume(SEMICOLON, "expected ';' after let statement")

	return &ast.LetStmt{
		Pos:    p.makePos(start),
		EndPos: p.makeEndPos(semi),
		Name:   name,
		Expr:   expr,
	}
}

func (p *Parser) parseReturnStmt() *ast.ReturnStmt {
	start := p.consume(RETURN, "expected 'return'")
	var value ast.Expr
	if !p.check(SEMICOLON) {
		value = p.parseExpr()
	}
	end := p.consume(SEMICOLON, "expected ';' after return statement")

	return &ast.ReturnStmt{
		Pos:    p.makePos(start),
		EndPos: p.makeEndPos(end),
		Value:  value,
	}
}

func (p *Parser) parseAssertStmt() *ast.AssertStmt {
	start := p.consume(ASSERT, "expected 'assert'")
	p.consume(BANG, "expected '!' after 'assert'")
	p.consume(LEFT_PAREN, "expected '(' after 'assert!'")

	var args []ast.Expr
	for {
		args = append(args, p.parseExpr())
		if !p.match(COMMA) {
			break
		}
	}

	end := p.consume(RIGHT_PAREN, "expected ')' to close assert arguments")
	p.consume(SEMICOLON, "expected ';' after assert statement")

	return &ast.AssertStmt{
		Pos:    p.makePos(start),
		EndPos: p.makeEndPos(end),
		Args:   args,
	}
}

func (p *Parser) parseExpr() ast.Expr {
	return p.parsePrattExpr(0)
}

func isAssignable(expr ast.Expr) bool {
	switch expr.(type) {
	case *ast.IdentExpr, *ast.FieldAccessExpr, *ast.UnaryExpr:
		return true
	default:
		return false
	}
}

func isAssignOperator(tok Token) bool {
	switch tok.Type {
	case EQUAL, PLUS_EQUAL, MINUS_EQUAL, STAR_EQUAL, SLASH_EQUAL, PERCENT_EQUAL:
		return true
	default:
		return false
	}
}

func assignOpFromToken(tok Token) ast.AssignType {
	switch tok.Type {
	case EQUAL:
		return ast.ASSIGN
	case PLUS_EQUAL:
		return ast.PLUS_ASSIGN
	case MINUS_EQUAL:
		return ast.MINUS_ASSIGN
	case STAR_EQUAL:
		return ast.STAR_ASSIGN
	case SLASH_EQUAL:
		return ast.SLASH_ASSIGN
	case PERCENT_EQUAL:
		return ast.PERCENT_ASSIGN
	default:
		return ast.ASSIGN
	}
}
