package parser

import "kanso/internal/ast"

func (p *Parser) parseUse() *ast.Use {
	startToken := p.consume(USE, "expected 'use' keyword")

	namespaces := []*ast.Namespace{}
	imports := []*ast.ImportItem{}

	// Parse namespace path: A::B::C
	for {
		if !p.check(IDENTIFIER) {
			p.errorAtCurrent("expected namespace identifier in use statement")
			p.synchronize()
			break
		}

		nameTok := p.advance()
		ns := &ast.Namespace{
			Pos:    p.makePos(nameTok),
			EndPos: p.makeEndPos(nameTok),
			Name: ast.Ident{
				Pos:    p.makePos(nameTok),
				EndPos: p.makeEndPos(nameTok),
				Value:  nameTok.Lexeme,
			},
		}
		namespaces = append(namespaces, ns)

		if p.match(DOUBLE_COLON) {
			if p.check(LEFT_BRACE) {
				break // into import item list
			}
			continue // continue parsing more namespace parts
		}
		break
	}

	// Parse optional { X, Y, Z }
	if p.match(LEFT_BRACE) {
		for {
			if !p.check(IDENTIFIER) {
				p.errorAtCurrent("expected identifier inside import list")
				p.synchronize()
				break
			}

			itemTok := p.advance()
			imp := &ast.ImportItem{
				Pos:    p.makePos(itemTok),
				EndPos: p.makeEndPos(itemTok),
				Name: ast.Ident{
					Pos:    p.makePos(itemTok),
					EndPos: p.makeEndPos(itemTok),
					Value:  itemTok.Lexeme,
				},
			}
			imports = append(imports, imp)

			if p.match(COMMA) {
				continue
			}
			break
		}

		if !p.match(RIGHT_BRACE) {
			p.errorAtCurrent("expected '}' to close import list")
			p.synchronize()
		}
	}

	endTok := p.consume(SEMICOLON, "expected ';' after use statement")
	if endTok.Type == ILLEGAL {
		p.synchronize()
	}

	return &ast.Use{
		Pos:        p.makePos(startToken),
		EndPos:     p.makeEndPos(endTok),
		Namespaces: namespaces,
		Imports:    imports,
	}
}
