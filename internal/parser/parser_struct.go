package parser

import "kanso/internal/ast"

func (p *Parser) parseStruct(attr *ast.Attribute) *ast.Struct {
	startToken := p.consume(STRUCT, "expected 'struct' keyword")

	// Parse struct name
	name, ok := p.consumeIdent("expected struct name")
	if !ok {
		p.synchronize()
		return nil
	}

	// Parse struct body
	items := p.parseStructBody()
	endToken := p.previous() // Set by parseStructBody

	return &ast.Struct{
		Pos:       p.makePos(startToken),
		EndPos:    p.makeEndPos(endToken),
		Attribute: attr,
		Name:      name,
		Items:     items,
	}
}

// parseStructBody parses the struct body between { and }
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

// parseStructField parses a single field: name: Type,
func (p *Parser) parseStructField() *ast.StructField {
	// Parse field name
	name, ok := p.consumeIdent("expected field name")
	if !ok {
		p.synchronize()
		return nil
	}

	// Parse field type
	p.consume(COLON, "expected ':' after field name")
	typ := p.parseVariableType()
	if typ == nil {
		p.synchronize()
		return nil
	}

	// Parse trailing comma (required in Kanso structs)
	end := p.consume(COMMA, "expected ',' after struct field")

	return &ast.StructField{
		Pos:          p.makePos(p.previous()), // Use name token position
		EndPos:       p.makeEndPos(end),
		Name:         name,
		VariableType: typ,
	}
}
