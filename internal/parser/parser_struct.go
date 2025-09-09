package parser

import "kanso/internal/ast"

func (p *Parser) parseStruct(attr *ast.Attribute) *ast.Struct {
	return p.parseStructWithDoc(attr, nil)
}

func (p *Parser) parseStructWithDoc(attr *ast.Attribute, docComment *ast.DocComment) *ast.Struct {
	startToken := p.consume(STRUCT, "expected 'struct' keyword")

	name, ok := p.consumeIdent("expected struct name")
	if !ok {
		p.synchronize()
		return nil
	}

	items := p.parseStructBody()
	endToken := p.previous() // parseStructBody leaves p at closing brace

	return &ast.Struct{
		Pos:        p.makePos(startToken),
		EndPos:     p.makeEndPos(endToken),
		Attribute:  attr,
		DocComment: docComment,
		Name:       name,
		Items:      items,
	}
}

func (p *Parser) parseStructBody() []ast.StructItem {
	p.consume(LEFT_BRACE, "expected '{' to start struct body")
	var items []ast.StructItem

	for !p.check(RIGHT_BRACE) && !p.isAtEnd() {
		if p.check(COMMENT) {
			items = append(items, p.parseComment())
			continue
		}

		field := p.parseStructField()
		if field != nil {
			items = append(items, field)
		} else {
			p.errorAtCurrent("expected struct field or comment")
			p.synchronize()
		}
	}

	p.consume(RIGHT_BRACE, "expected '}' to close struct body")
	return items
}

func (p *Parser) parseStructField() *ast.StructField {
	name, ok := p.consumeIdent("expected field name")
	if !ok {
		p.synchronize()
		return nil
	}

	p.consume(COLON, "expected ':' after field name")
	typ := p.parseVariableType()
	if typ == nil {
		p.synchronize()
		return nil
	}

	// Kanso requires trailing commas for consistency
	end := p.consume(COMMA, "expected ',' after struct field")

	return &ast.StructField{
		Pos:          p.makePos(p.previous()), // position from name token not field type
		EndPos:       p.makeEndPos(end),
		Name:         name,
		VariableType: typ,
	}
}
