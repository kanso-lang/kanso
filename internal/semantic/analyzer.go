package semantic

import (
	"kanso/internal/ast"
)

type Analyzer struct {
	contract *ast.Contract
	errors   []SemanticError
	symbols  *SymbolTable
}

type SemanticError struct {
	Message  string
	Position ast.Position
}

func NewAnalyzer() *Analyzer {
	return &Analyzer{
		errors: make([]SemanticError, 0),
	}
}

func (a *Analyzer) Analyze(contract *ast.Contract) []SemanticError {
	a.contract = contract
	a.errors = make([]SemanticError, 0)
	a.symbols = NewSymbolTable(nil)

	a.analyzeContract(contract)

	return a.errors
}

func (a *Analyzer) analyzeContract(contract *ast.Contract) {
	moduleCount := 0
	for _, item := range contract.ContractItems {
		if module, ok := item.(*ast.Module); ok {
			moduleCount++
			a.analyzeModule(module)
		}
	}

	if moduleCount == 0 {
		a.addError("contract must have at least one module", ast.Position{})
	}
}

func (a *Analyzer) analyzeModule(module *ast.Module) {
	if len(module.Attributes) == 0 {
		a.addError("module must have at least one attribute", module.NodePos())
		return
	}

	for _, item := range module.ModuleItems {
		switch node := item.(type) {
		case *ast.Function:
			a.analyzeFunction(node)
		case *ast.Struct:
			a.analyzeStruct(node)
		}
	}
}

func (a *Analyzer) analyzeFunction(fn *ast.Function) {
	if existing := a.symbols.LookupLocal(fn.Name.Value); existing != nil {
		a.addError("duplicate declaration: "+fn.Name.Value, fn.NodePos())
		return
	}

	a.symbols.Define(fn.Name.Value, SymbolFunction, fn, fn.NodePos())
}

func (a *Analyzer) analyzeStruct(s *ast.Struct) {
	if existing := a.symbols.LookupLocal(s.Name.Value); existing != nil {
		a.addError("duplicate declaration: "+s.Name.Value, s.NodePos())
		return
	}

	a.symbols.Define(s.Name.Value, SymbolStruct, s, s.NodePos())
}

func (a *Analyzer) addError(message string, pos ast.Position) {
	a.errors = append(a.errors, SemanticError{
		Message:  message,
		Position: pos,
	})
}
