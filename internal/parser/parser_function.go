package parser

import (
	"fmt"
	"kanso/internal/ast"
)

func (p *Parser) parseFunction(attr *ast.Attribute, isExternal bool) *ast.Function {
	return p.parseFunctionWithDoc(attr, isExternal, nil)
}

func (p *Parser) parseFunctionWithDoc(attr *ast.Attribute, isExternal bool, docComment *ast.DocComment) *ast.Function {
	startToken := p.consume(FN, "expected 'fn' keyword")

	name, ok := p.consumeIdent("expected function name")
	if !ok {
		p.synchronize()
		return nil
	}

	params := p.parseFunctionParameters()
	returnType := p.parseFunctionReturnType()
	reads := p.parseFunctionReadsClause()
	writes := p.parseFunctionWritesClause()
	body := p.parseFunctionBlock()
	if body.Pos == (ast.Position{}) { // parser recovery failed
		p.synchronize()
		return nil
	}

	return &ast.Function{
		Pos:        p.makePos(startToken),
		EndPos:     body.EndPos,
		Attribute:  attr,
		DocComment: docComment,
		External:   isExternal,
		Name:       name,
		Params:     params,
		Return:     returnType,
		Reads:      reads,
		Writes:     writes,
		Body:       &body,
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

// parseFunctionReturnType parses the optional return type after '->'
func (p *Parser) parseFunctionReturnType() *ast.VariableType {
	if p.match(ARROW) {
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
		} else if p.check(REQUIRE) {
			stmt := p.parseRequireStmt()
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

				// Use improved semicolon error recovery for assignments
				endPos := p.consumeSemicolonWithBetterRecovery(value.NodeEndPos(), "assignment")

				items = append(items, &ast.AssignStmt{
					Pos:      expr.NodePos(),
					EndPos:   endPos,
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
				// Create a better error message that points to the missing semicolon
				// Report the error at the END of the expression where the semicolon should be
				errorMsg := fmt.Sprintf("missing ';' after %s statement", getExpressionType(expr))
				p.errorAtPosition(errorMsg, expr.NodeEndPos())

				// Consume the token anyway to continue parsing
				semi := p.advance() // Just advance without reporting another error
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

	// Check for mut keyword
	mut := p.match(MUT)

	name, ok := p.consumeIdent("expected variable name after 'let'")
	if !ok {
		return nil
	}

	p.consume(EQUAL, "expected '=' in let statement")
	expr := p.parseExpr()

	// Use improved semicolon error recovery
	semiEndPos := p.consumeSemicolonWithBetterRecovery(expr.NodeEndPos(), "let")

	return &ast.LetStmt{
		Pos:    p.makePos(start),
		EndPos: semiEndPos,
		Mut:    mut,
		Name:   name,
		Expr:   expr,
	}
}

func (p *Parser) parseReturnStmt() *ast.ReturnStmt {
	start := p.consume(RETURN, "expected 'return'")
	var value ast.Expr
	var endPos ast.Position

	if !p.check(SEMICOLON) {
		value = p.parseExpr()
		endPos = value.NodeEndPos()
	} else {
		endPos = p.makeEndPos(start)
	}

	// Use improved semicolon error recovery
	endPos = p.consumeSemicolonWithBetterRecovery(endPos, "return")

	return &ast.ReturnStmt{
		Pos:    p.makePos(start),
		EndPos: endPos,
		Value:  value,
	}
}

func (p *Parser) parseRequireStmt() *ast.RequireStmt {
	start := p.consume(REQUIRE, "expected 'require'")
	p.consume(BANG, "expected '!' after 'require'")
	p.consume(LEFT_PAREN, "expected '(' after 'require!'")

	var args []ast.Expr
	for {
		args = append(args, p.parseExpr())
		if !p.match(COMMA) {
			break
		}
	}

	end := p.consume(RIGHT_PAREN, "expected ')' to close require arguments")

	// Use improved semicolon error recovery
	semiEndPos := p.consumeSemicolonWithBetterRecovery(p.makeEndPos(end), "require")

	return &ast.RequireStmt{
		Pos:    p.makePos(start),
		EndPos: semiEndPos,
		Args:   args,
	}
}

func (p *Parser) parseExpr() ast.Expr {
	return p.parsePrattExpr(0)
}

func isAssignable(expr ast.Expr) bool {
	switch expr.(type) {
	case *ast.IdentExpr, *ast.FieldAccessExpr, *ast.UnaryExpr, *ast.IndexExpr:
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

// getExpressionType returns a human-readable description of the expression type
// for better error messages
func getExpressionType(expr ast.Expr) string {
	switch e := expr.(type) {
	case *ast.CallExpr:
		if ident, ok := e.Callee.(*ast.IdentExpr); ok {
			if ident.Name == "emit" {
				return "'emit()' call"
			}
			return fmt.Sprintf("'%s()' call", ident.Name)
		}
		return "function call"
	case *ast.IdentExpr:
		return fmt.Sprintf("'%s'", e.Name)
	case *ast.FieldAccessExpr:
		return "field access"
	case *ast.IndexExpr:
		return "index access"
	case *ast.LiteralExpr:
		return "literal"
	case *ast.UnaryExpr:
		return "unary expression"
	case *ast.BinaryExpr:
		return "binary expression"
	default:
		return "expression"
	}
}
