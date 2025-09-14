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
	Name            string
	Kind            SymbolKind
	Node            ast.Node
	Position        ast.Position
	Type            *stdlib.TypeRef // Enables type checking for variable usage
	Mutable         bool            // Enforces Rust-like immutability-by-default semantics
	Used            bool            // Tracks if variable is ever read
	Modified        bool            // Tracks if mutable variable is ever modified
	ReadAfterModify bool            // Tracks if variable is read after modification
	LastModifyPos   ast.Position    // Position of the last modification
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

func (st *SymbolTable) MarkVariableUsed(name string) {
	if symbol := st.Lookup(name); symbol != nil && symbol.Kind == SymbolVariable {
		symbol.Used = true
	}
}

func (st *SymbolTable) MarkVariableModified(name string) {
	if symbol := st.Lookup(name); symbol != nil && symbol.Kind == SymbolVariable {
		symbol.Modified = true
		// Reset ReadAfterModify - it will be set again if variable is read after this modification
		symbol.ReadAfterModify = false
		// Note: LastModifyPos will be set by MarkVariableModifiedAt
	}
}

func (st *SymbolTable) MarkVariableModifiedAt(name string, pos ast.Position) {
	if symbol := st.Lookup(name); symbol != nil && symbol.Kind == SymbolVariable {
		symbol.Modified = true
		// Reset ReadAfterModify - it will be set again if variable is read after this modification
		symbol.ReadAfterModify = false
		symbol.LastModifyPos = pos
	}
}

func (st *SymbolTable) MarkVariableReadAfterModify(name string) {
	if symbol := st.Lookup(name); symbol != nil && symbol.Kind == SymbolVariable && symbol.Modified {
		symbol.ReadAfterModify = true
	}
}
