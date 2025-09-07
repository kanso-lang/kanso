package semantic

import (
	"kanso/internal/ast"
)

var validModuleAttributes = map[string]bool{
	"contract": true,
}

var validStructAttributes = map[string]bool{
	"event":   true,
	"storage": true,
}

var validFunctionAttributes = map[string]bool{
	"create": true,
}

type Analyzer struct {
	contract *ast.Contract
	errors   []SemanticError
	symbols  *SymbolTable
	context  *ContextRegistry
}

type SemanticError struct {
	Message  string
	Position ast.Position
}

func NewAnalyzer() *Analyzer {
	return &Analyzer{
		errors:  make([]SemanticError, 0),
		context: NewContextRegistry(),
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

	a.validateModuleAttributes(module)

	// First pass: process use statements and collect types
	storageStructs := make(map[string]bool)

	for _, item := range module.ModuleItems {
		switch node := item.(type) {
		case *ast.Use:
			// Process import statements
			importErrors := a.context.ProcessUseStatement(node)
			for _, err := range importErrors {
				a.addError(err, node.NodePos())
			}
		case *ast.Struct:
			// Add user-defined structs to context
			a.context.AddUserDefinedType(node.Name.Value, node)
			if node.Attribute != nil && node.Attribute.Name == "storage" {
				storageStructs[node.Name.Value] = true
			}
		}
	}

	// Second pass: analyze all items
	var createFunction *ast.Function
	for _, item := range module.ModuleItems {
		switch node := item.(type) {
		case *ast.Function:
			a.analyzeFunction(node)
			a.validateFunctionReadsWrites(node, storageStructs)
			if node.Attribute != nil && node.Attribute.Name == "create" {
				if createFunction != nil {
					a.addError("multiple functions with #[create] attribute found", node.NodePos())
				} else {
					createFunction = node
				}
				a.validateConstructor(node, storageStructs)
			}
		case *ast.Struct:
			a.analyzeStruct(node)
		case *ast.Use:
			// Already processed in first pass
		}
	}
}

func (a *Analyzer) analyzeFunction(fn *ast.Function) {
	if existing := a.symbols.LookupLocal(fn.Name.Value); existing != nil {
		a.addError("duplicate declaration: "+fn.Name.Value, fn.NodePos())
		return
	}

	a.validateFunctionAttributes(fn)
	a.symbols.Define(fn.Name.Value, SymbolFunction, fn, fn.NodePos())
}

func (a *Analyzer) analyzeStruct(s *ast.Struct) {
	if existing := a.symbols.LookupLocal(s.Name.Value); existing != nil {
		a.addError("duplicate declaration: "+s.Name.Value, s.NodePos())
		return
	}

	a.validateStructAttributes(s)
	a.symbols.Define(s.Name.Value, SymbolStruct, s, s.NodePos())
}

func (a *Analyzer) validateModuleAttributes(module *ast.Module) {
	for _, attr := range module.Attributes {
		if !validModuleAttributes[attr.Name] {
			a.addError("invalid module attribute: "+attr.Name, attr.NodePos())
		}
	}
}

func (a *Analyzer) validateStructAttributes(s *ast.Struct) {
	if s.Attribute != nil {
		if !validStructAttributes[s.Attribute.Name] {
			a.addError("invalid struct attribute: "+s.Attribute.Name, s.Attribute.NodePos())
		}
	}
}

func (a *Analyzer) validateFunctionAttributes(fn *ast.Function) {
	if fn.Attribute != nil {
		if !validFunctionAttributes[fn.Attribute.Name] {
			a.addError("invalid function attribute: "+fn.Attribute.Name, fn.Attribute.NodePos())
		}
	}
}

func (a *Analyzer) validateFunctionReadsWrites(fn *ast.Function, storageStructs map[string]bool) {
	// Validate reads clauses reference storage structs
	readStructs := make(map[string]bool)
	for _, read := range fn.Reads {
		if !storageStructs[read.Value] {
			a.addError("reads clause references non-storage struct: "+read.Value, read.NodePos())
			continue
		}

		// Check for duplicate reads
		if readStructs[read.Value] {
			a.addError("duplicate reads clause for struct: "+read.Value, read.NodePos())
		}
		readStructs[read.Value] = true
	}

	// Validate writes clauses reference storage structs
	writeStructs := make(map[string]bool)
	for _, write := range fn.Writes {
		if !storageStructs[write.Value] {
			a.addError("writes clause references non-storage struct: "+write.Value, write.NodePos())
			continue
		}

		// Check for duplicate writes
		if writeStructs[write.Value] {
			a.addError("duplicate writes clause for struct: "+write.Value, write.NodePos())
		}
		writeStructs[write.Value] = true

		// Check for conflicting read + write to same struct
		if readStructs[write.Value] {
			a.addError("conflicting reads and writes clause for struct (write implies read): "+write.Value, write.NodePos())
		}
	}
}

func (a *Analyzer) validateConstructor(fn *ast.Function, storageStructs map[string]bool) {
	// Constructors must not have a return type
	if fn.Return != nil {
		a.addError("constructor functions cannot have a return type", fn.NodePos())
	}

	// Constructors must have a writes clause
	if len(fn.Writes) == 0 {
		a.addError("constructor functions must have a writes clause", fn.NodePos())
	} else {
		// Check if constructor writes to at least one storage struct
		hasStorageWrite := false
		for _, write := range fn.Writes {
			if storageStructs[write.Value] {
				hasStorageWrite = true
				break
			}
		}
		if !hasStorageWrite {
			a.addError("constructor functions must write to a storage struct", fn.NodePos())
		}
	}
}

func (a *Analyzer) addError(message string, pos ast.Position) {
	a.errors = append(a.errors, SemanticError{
		Message:  message,
		Position: pos,
	})
}
