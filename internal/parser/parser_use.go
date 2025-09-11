package parser

import "kanso/internal/ast"

func (p *Parser) parseUse() *ast.Use {
	startToken := p.consume(USE, "expected 'use' keyword")

	namespaces := []*ast.Namespace{}
	imports := []*ast.ImportItem{}

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
				break // transition to import list parsing
			}
			continue // more namespace segments to parse
		}
		break
	}

	// Handle brace-enclosed import lists like {sender, emit}
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

	// Use improved semicolon error recovery
	endPos := p.consumeSemicolonWithBetterRecovery(p.makeEndPos(p.previous()), "use")

	return &ast.Use{
		Pos:        p.makePos(startToken),
		EndPos:     endPos,
		Namespaces: namespaces,
		Imports:    imports,
	}
}
