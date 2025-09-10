package semantic

import (
	"kanso/internal/ast"
	"kanso/internal/stdlib"
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
	Type     *stdlib.TypeRef // Enables type checking for variable usage
	Mutable  bool            // Enforces Rust-like immutability-by-default semantics
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

func (st *SymbolTable) DefineWithType(name string, kind SymbolKind, node ast.Node, pos ast.Position, symbolType *stdlib.TypeRef) *Symbol {
	symbol := &Symbol{
		Name:     name,
		Kind:     kind,
		Node:     node,
		Position: pos,
		Type:     symbolType,
	}
	st.symbols[name] = symbol
	return symbol
}

func (st *SymbolTable) DefineVariable(name string, node ast.Node, pos ast.Position, symbolType *stdlib.TypeRef, mutable bool) *Symbol {
	symbol := &Symbol{
		Name:     name,
		Kind:     SymbolVariable,
		Node:     node,
		Position: pos,
		Type:     symbolType,
		Mutable:  mutable,
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
