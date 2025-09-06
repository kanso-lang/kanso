package semantic

import (
	"kanso/internal/ast"
)

type SymbolKind int

const (
	SymbolFunction SymbolKind = iota
	SymbolStruct
	SymbolParameter
	SymbolVariable
)

type Symbol struct {
	Name     string
	Kind     SymbolKind
	Node     ast.Node
	Position ast.Position
}

type SymbolTable struct {
	symbols map[string]*Symbol
	parent  *SymbolTable
}

func NewSymbolTable(parent *SymbolTable) *SymbolTable {
	return &SymbolTable{
		symbols: make(map[string]*Symbol),
		parent:  parent,
	}
}

func (st *SymbolTable) Define(name string, kind SymbolKind, node ast.Node, pos ast.Position) *Symbol {
	symbol := &Symbol{
		Name:     name,
		Kind:     kind,
		Node:     node,
		Position: pos,
	}
	st.symbols[name] = symbol
	return symbol
}

func (st *SymbolTable) Lookup(name string) *Symbol {
	if symbol, exists := st.symbols[name]; exists {
		return symbol
	}
	if st.parent != nil {
		return st.parent.Lookup(name)
	}
	return nil
}

func (st *SymbolTable) LookupLocal(name string) *Symbol {
	if symbol, exists := st.symbols[name]; exists {
		return symbol
	}
	return nil
}
