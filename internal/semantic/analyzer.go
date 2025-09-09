package semantic

import (
	"fmt"
	"kanso/internal/ast"
	"kanso/internal/stdlib"
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
			a.analyzeFunctionBody(node) // Add expression analysis
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

func (a *Analyzer) analyzeFunctionBody(fn *ast.Function) {
	if fn.Body == nil {
		return // No body to analyze (e.g., function declarations)
	}

	// Analyze all items in the function body
	for _, item := range fn.Body.Items {
		a.analyzeFunctionBlockItem(item)
	}

	// Analyze tail expression if present
	if fn.Body.TailExpr != nil {
		a.analyzeExpression(fn.Body.TailExpr.Expr)
	}
}

func (a *Analyzer) analyzeFunctionBlockItem(item ast.FunctionBlockItem) {
	switch node := item.(type) {
	case *ast.ExprStmt:
		a.analyzeExpression(node.Expr)
	case *ast.LetStmt:
		if node.Expr != nil {
			a.analyzeExpression(node.Expr)
		}
	case *ast.ReturnStmt:
		if node.Value != nil {
			a.analyzeExpression(node.Value)
		}
	case *ast.AssertStmt:
		// Assert can have multiple arguments
		for _, arg := range node.Args {
			a.analyzeExpression(arg)
		}
	case *ast.AssignStmt:
		a.analyzeExpression(node.Target)
		a.analyzeExpression(node.Value)
		// Add other statement types as needed
	}
}

func (a *Analyzer) analyzeExpression(expr ast.Expr) {
	if expr == nil {
		return
	}

	switch node := expr.(type) {
	case *ast.CallExpr:
		a.analyzeCallExpression(node)
	case *ast.FieldAccessExpr:
		a.analyzeExpression(node.Target)
	case *ast.StructLiteralExpr:
		for _, field := range node.Fields {
			a.analyzeExpression(field.Value)
		}
	case *ast.ParenExpr:
		a.analyzeExpression(node.Value)
	case *ast.BinaryExpr:
		a.analyzeExpression(node.Left)
		a.analyzeExpression(node.Right)
	case *ast.UnaryExpr:
		a.analyzeExpression(node.Value)
		// Add other expression types as needed
	}
}

func (a *Analyzer) analyzeCallExpression(call *ast.CallExpr) {
	// Analyze arguments first
	for _, arg := range call.Args {
		a.analyzeExpression(arg)
	}

	// Determine call type and validate
	switch callee := call.Callee.(type) {
	case *ast.IdentExpr:
		// Direct function call like sender()
		a.validateDirectFunctionCall(callee.Name, call)
	case *ast.CalleePath:
		// Check if it's a single-part path (direct function call) or multi-part (module call)
		if len(callee.Parts) == 1 {
			// Single identifier like sender() - treat as direct function call
			a.validateDirectFunctionCall(callee.Parts[0].Value, call)
		} else {
			// Multi-part path like Table::empty() or errors::invalid_argument()
			a.validateModuleFunctionCall(callee, call)
		}
	default:
		// Other callee types (field access, etc.)
		a.analyzeExpression(call.Callee)
	}
}

func (a *Analyzer) validateDirectFunctionCall(functionName string, call *ast.CallExpr) {
	// Check if function is imported
	if !a.context.IsImportedFunction(functionName) {
		a.addError(fmt.Sprintf("function '%s' is not imported or defined", functionName), call.NodePos())
		return
	}

	// Get function definition for parameter validation
	funcDef := a.context.GetFunctionDefinition(functionName)
	if funcDef == nil {
		a.addError(fmt.Sprintf("function '%s' definition not found", functionName), call.NodePos())
		return
	}

	// Validate parameter count
	if len(call.Args) != len(funcDef.Parameters) {
		a.addError(fmt.Sprintf("function '%s' expects %d arguments, got %d",
			functionName, len(funcDef.Parameters), len(call.Args)), call.NodePos())
		return
	}

	// Validate parameter types
	for i, arg := range call.Args {
		expectedType := funcDef.Parameters[i].Type
		if !a.validateArgumentType(arg, expectedType, call.NodePos()) {
			// Error already added in validateArgumentType
			continue
		}
	}
}

func (a *Analyzer) validateModuleFunctionCall(callee *ast.CalleePath, call *ast.CallExpr) {
	if len(callee.Parts) != 2 {
		a.addError("invalid module function call format", call.NodePos())
		return
	}

	moduleName := callee.Parts[0].Value
	functionName := callee.Parts[1].Value

	// Check if module is imported
	if !a.context.IsImportedModule(moduleName) {
		a.addError(fmt.Sprintf("module '%s' is not imported", moduleName), call.NodePos())
		return
	}

	// Get function definition from module
	funcDef := a.context.GetModuleFunctionDefinition(moduleName, functionName)
	if funcDef == nil {
		a.addError(fmt.Sprintf("function '%s' not found in module '%s'", functionName, moduleName), call.NodePos())
		return
	}

	// Validate parameter count
	if len(call.Args) != len(funcDef.Parameters) {
		a.addError(fmt.Sprintf("function '%s::%s' expects %d arguments, got %d",
			moduleName, functionName, len(funcDef.Parameters), len(call.Args)), call.NodePos())
		return
	}

	// Validate parameter types
	for i, arg := range call.Args {
		expectedType := funcDef.Parameters[i].Type
		if !a.validateArgumentType(arg, expectedType, call.NodePos()) {
			// Error already added in validateArgumentType
			continue
		}
	}

	// TODO: Handle generics (next step)
}

// validateArgumentType validates that an argument expression matches the expected parameter type
func (a *Analyzer) validateArgumentType(arg ast.Expr, expectedType *stdlib.TypeRef, pos ast.Position) bool {
	// Get the inferred type of the argument expression
	argType := a.inferExpressionType(arg)
	if argType == nil {
		// Cannot infer type - for now, allow it (could be improved later)
		return true
	}

	// Check if types match
	if !a.typesMatch(argType, expectedType) {
		a.addError(fmt.Sprintf("argument type %s does not match expected type %s",
			a.typeToString(argType), a.typeToString(expectedType)), pos)
		return false
	}

	return true
}

// inferExpressionType attempts to infer the type of an expression
func (a *Analyzer) inferExpressionType(expr ast.Expr) *stdlib.TypeRef {
	switch node := expr.(type) {
	case *ast.LiteralExpr:
		return a.inferLiteralType(node.Value)
	case *ast.IdentExpr:
		// Check for boolean literals
		if node.Name == "true" || node.Name == "false" {
			return stdlib.BoolType()
		}
		// Look up variable or function
		// For now, return nil (needs symbol table integration)
		return nil
	case *ast.CallExpr:
		// Function call - get return type
		return a.inferCallExpressionType(node)
	case *ast.FieldAccessExpr:
		// Field access - needs struct type info
		return nil
	case *ast.BinaryExpr:
		// Binary operation - needs operator type rules
		return nil
	case *ast.UnaryExpr:
		// Unary operation - needs operator type rules
		return nil
	case *ast.ParenExpr:
		// Parenthesized expression - same as inner type
		return a.inferExpressionType(node.Value)
	default:
		return nil
	}
}

// inferLiteralType infers the type of a literal value
func (a *Analyzer) inferLiteralType(value string) *stdlib.TypeRef {
	// Simple literal type inference
	if value == "true" || value == "false" {
		return stdlib.BoolType()
	}
	if value == "0x0" {
		return stdlib.AddressType()
	}
	// For now, assume numeric literals are u64 (matches most stdlib functions)
	// This could be improved with proper literal parsing and type inference
	return stdlib.U64Type()
}

// inferCallExpressionType infers the return type of a function call
func (a *Analyzer) inferCallExpressionType(call *ast.CallExpr) *stdlib.TypeRef {
	switch callee := call.Callee.(type) {
	case *ast.IdentExpr:
		if funcDef := a.context.GetFunctionDefinition(callee.Name); funcDef != nil {
			return funcDef.ReturnType
		}
	case *ast.CalleePath:
		if len(callee.Parts) == 1 {
			// Direct function call
			if funcDef := a.context.GetFunctionDefinition(callee.Parts[0].Value); funcDef != nil {
				return funcDef.ReturnType
			}
		} else if len(callee.Parts) == 2 {
			// Module function call
			moduleName := callee.Parts[0].Value
			functionName := callee.Parts[1].Value
			if funcDef := a.context.GetModuleFunctionDefinition(moduleName, functionName); funcDef != nil {
				return funcDef.ReturnType
			}
		}
	}
	return nil
}

// typesMatch checks if two types are compatible
func (a *Analyzer) typesMatch(actual, expected *stdlib.TypeRef) bool {
	if actual == nil || expected == nil {
		return actual == expected
	}

	// Handle generic type parameters (they match anything for now)
	if expected.IsGeneric {
		return true
	}

	// Simple name matching for non-generic types
	if actual.Name != expected.Name {
		return false
	}

	// For generic types, would need to match type arguments
	// For now, just check basic type name equality
	return true
}

// typeToString converts a type reference to a string for error messages
func (a *Analyzer) typeToString(typeRef *stdlib.TypeRef) string {
	if typeRef == nil {
		return "unknown"
	}

	if typeRef.IsGeneric {
		return typeRef.Name // Generic parameter like T, K, V
	}

	if len(typeRef.GenericArgs) == 0 {
		return typeRef.Name // Simple type like u256, address
	}

	// Generic type with arguments like Table<K, V>
	result := typeRef.Name + "<"
	for i, arg := range typeRef.GenericArgs {
		if i > 0 {
			result += ", "
		}
		result += a.typeToString(arg)
	}
	result += ">"
	return result
}

func (a *Analyzer) addError(message string, pos ast.Position) {
	a.errors = append(a.errors, SemanticError{
		Message:  message,
		Position: pos,
	})
}
