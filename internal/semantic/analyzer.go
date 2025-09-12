package semantic

import (
	"fmt"
	"kanso/internal/ast"
	"kanso/internal/errors"
	"kanso/internal/stdlib"
)

// Attribute validation maps ensure only semantically meaningful attributes are accepted.
// This prevents typos and enforces the contract domain model where storage structs
// represent persistent state, events represent blockchain logs, and create functions
// are constructors with special initialization semantics.
var validModuleAttributes = map[string]bool{
	"contract": true,
}

var validStructAttributes = map[string]bool{
	"event":   true, // Structs that will be emitted to blockchain logs
	"storage": true, // Structs representing persistent contract state
}

var validFunctionAttributes = map[string]bool{
	"create": true, // Constructor functions with initialization-only semantics
}

type Analyzer struct {
	contract       *ast.Contract
	errors         []errors.CompilerError   // All errors with suggestions and proper formatting
	symbols        *SymbolTable             // Tracks variable/function scoping within contract
	context        *ContextRegistry         // Manages imports and standard library integration
	localFunctions map[string]*ast.Function // Tracks functions defined in this contract
}

func NewAnalyzer() *Analyzer {
	return &Analyzer{
		errors:         make([]errors.CompilerError, 0),
		context:        NewContextRegistry(),
		localFunctions: make(map[string]*ast.Function),
	}
}

// SemanticError provides backward compatibility with tests
type SemanticError struct {
	Message  string
	Position ast.Position
}

func (a *Analyzer) Analyze(contract *ast.Contract) []SemanticError {
	a.contract = contract
	a.errors = make([]errors.CompilerError, 0)
	a.localFunctions = make(map[string]*ast.Function) // Reset for each analysis
	a.symbols = NewSymbolTable(nil)                   // Root scope for contract-level declarations

	a.analyzeContract(contract)

	// Convert errors to SemanticError format for test compatibility
	compatibilityErrors := make([]SemanticError, len(a.errors))
	for i, err := range a.errors {
		compatibilityErrors[i] = SemanticError{
			Message:  err.Message,
			Position: err.Position,
		}
	}
	return compatibilityErrors
}

// GetErrors returns all errors with suggestions and proper formatting
func (a *Analyzer) GetErrors() []errors.CompilerError {
	return a.errors
}

func (a *Analyzer) analyzeContract(contract *ast.Contract) {
	// Two-pass analysis prevents forward reference errors: struct definitions must be processed
	// before functions that reference them in reads/writes clauses can be validated
	storageStructs := make(map[string]bool)

	// License comments and documentation are semantically significant for contract metadata
	allItems := make([]ast.ContractItem, 0, len(contract.LeadingComments)+len(contract.Items))
	allItems = append(allItems, contract.LeadingComments...)
	allItems = append(allItems, contract.Items...)

	// Pass 1: Build symbol tables and type context before cross-reference validation
	for _, item := range allItems {
		switch node := item.(type) {
		case *ast.Use:
			importErrors := a.context.ProcessUseStatement(node)
			for _, err := range importErrors {
				a.addError(err, node.NodePos())
			}
		case *ast.Struct:
			a.context.AddUserDefinedType(node.Name.Value, node)
			// Only storage structs represent persistent state that functions can declare access to
			if node.Attribute != nil && node.Attribute.Name == "storage" {
				storageStructs[node.Name.Value] = true
			}
		case *ast.Function:
			// Local function registry enables validation of internal function calls
			a.localFunctions[node.Name.Value] = node
		}
	}

	// Second pass: validate function signatures and bodies with full type context
	var createFunction *ast.Function
	for _, item := range allItems {
		switch node := item.(type) {
		case *ast.Function:
			a.analyzeFunction(node)
			a.validateFunctionReadsWrites(node, storageStructs)
			a.analyzeFunctionBody(node)

			// Enforce blockchain contract constraint: exactly one constructor
			if node.Attribute != nil && node.Attribute.Name == "create" {
				if createFunction != nil {
					a.addError("multiple functions with #[create] attribute found", node.NodePos())
				} else {
					createFunction = node
				}
			}
		case *ast.Struct:
			a.analyzeStruct(node)
		}
	}
}

func (a *Analyzer) analyzeFunction(fn *ast.Function) {
	if existing := a.symbols.LookupLocal(fn.Name.Value); existing != nil {
		a.addCompilerError(errors.DuplicateDeclaration(fn.Name.Value, fn.NodePos()))
		return
	}

	a.validateFunctionAttributes(fn)
	a.validateConstructorConstraints(fn)
	a.symbols.Define(fn.Name.Value, SymbolFunction, fn, fn.NodePos())
}

func (a *Analyzer) analyzeStruct(s *ast.Struct) {
	if existing := a.symbols.LookupLocal(s.Name.Value); existing != nil {
		a.addCompilerError(errors.DuplicateDeclaration(s.Name.Value, s.NodePos()))
		return
	}

	a.validateStructAttributes(s)
	// Structs should have type information so they can be used in field access expressions
	structType := &stdlib.TypeRef{Name: s.Name.Value, IsGeneric: false}
	a.symbols.DefineWithType(s.Name.Value, SymbolStruct, s, s.NodePos(), structType)
}

func (a *Analyzer) validateStructAttributes(s *ast.Struct) {
	if s.Attribute != nil {
		if !validStructAttributes[s.Attribute.Name] {
			a.addCompilerError(errors.InvalidAttribute(s.Attribute.Name, s.Attribute.NodePos()))
		}
	}
}

func (a *Analyzer) validateFunctionAttributes(fn *ast.Function) {
	if fn.Attribute != nil {
		if !validFunctionAttributes[fn.Attribute.Name] {
			a.addCompilerError(errors.InvalidAttribute(fn.Attribute.Name, fn.Attribute.NodePos()))
		}
	}
}

func (a *Analyzer) validateConstructorConstraints(fn *ast.Function) {
	isConstructor := fn.Attribute != nil && fn.Attribute.Name == "create"

	if isConstructor {
		// Blockchain constructors run exactly once during deployment and cannot be called again,
		// so returning values would be meaningless since there's no caller to receive them
		if fn.Return != nil {
			a.addCompilerError(errors.InvalidConstructor("constructor functions cannot have a return type", fn.Return.NodePos()))
		}

		// Smart contracts must initialize persistent state during deployment or they're essentially useless,
		// so we require constructors to declare which storage they'll modify upfront for gas optimization
		if len(fn.Writes) == 0 {
			a.addCompilerError(errors.InvalidConstructor("constructor functions must have a writes clause", fn.NodePos()))
		} else {
			a.validateWritesReferences(fn.Writes)

			// A constructor that doesn't write to any storage struct serves no purpose in blockchain context
			// since contract deployment is expensive and should establish meaningful initial state
			hasStorageWrite := false
			for _, write := range fn.Writes {
				structType := a.context.GetUserDefinedType(write.Value)
				if structType != nil && structType.Attribute != nil && structType.Attribute.Name == "storage" {
					hasStorageWrite = true
					break
				}
			}
			if !hasStorageWrite {
				a.addCompilerError(errors.InvalidConstructor("constructor functions must write to a storage struct", fn.NodePos()))
			}
		}
	} else {
		// Regular functions still need storage access validation for gas estimation and security analysis
		if len(fn.Writes) > 0 {
			a.validateWritesReferences(fn.Writes)
		}
		if len(fn.Reads) > 0 {
			a.validateReadsReferences(fn.Reads)
		}
	}
}

func (a *Analyzer) validateWritesReferences(writes []ast.Ident) {
	for _, structRef := range writes {
		structName := structRef.Value
		structType := a.context.GetUserDefinedType(structName)

		// Only storage structs represent persistent blockchain state that can be modified.
		// Non-storage structs (events, regular structs) are immutable or ephemeral,
		// so allowing writes to them would be semantically meaningless
		if structType == nil || structType.Attribute == nil || structType.Attribute.Name != "storage" {
			a.addCompilerError(errors.InvalidReadsWrites(fmt.Sprintf("writes clause references non-storage struct: %s", structName), structRef.NodePos()))
		}
	}
}

func (a *Analyzer) validateReadsReferences(reads []ast.Ident) {
	for _, structRef := range reads {
		structName := structRef.Value
		structType := a.context.GetUserDefinedType(structName)

		// Reads clauses declare upfront which storage will be accessed.
		if structType == nil || structType.Attribute == nil || structType.Attribute.Name != "storage" {
			a.addCompilerError(errors.InvalidReadsWrites(fmt.Sprintf("reads clause references non-storage struct: %s", structName), structRef.NodePos()))
		}
	}
}

func (a *Analyzer) validateFunctionReadsWrites(fn *ast.Function, storageStructs map[string]bool) {
	// Reads/writes validation prevents accidental state access patterns
	readStructs := make(map[string]bool)
	for _, read := range fn.Reads {
		if readStructs[read.Value] {
			a.addError("duplicate reads clause for struct: "+read.Value, read.NodePos())
		}
		readStructs[read.Value] = true
	}

	writeStructs := make(map[string]bool)
	for _, write := range fn.Writes {
		if writeStructs[write.Value] {
			a.addError("duplicate writes clause for struct: "+write.Value, write.NodePos())
		}
		writeStructs[write.Value] = true

		// Writing to storage requires implies reading access as well
		if readStructs[write.Value] {
			a.addError("conflicting reads and writes clause for struct (write implies read): "+write.Value, write.NodePos())
		}
	}

	// TODO the complete call path analysis to ensure all storage accesses are declared
}

func (a *Analyzer) analyzeFunctionBody(fn *ast.Function) {
	if fn.Body == nil {
		return // No body to analyze (e.g., function declarations)
	}

	// Create a new scope for function body
	functionScope := NewSymbolTable(a.symbols)
	previousScope := a.symbols
	a.symbols = functionScope

	// Add function parameters to scope
	for _, param := range fn.Params {
		paramType := a.resolveVariableType(param.Type)
		if paramType != nil {
			a.symbols.DefineWithType(param.Name.Value, SymbolParameter, param, param.NodePos(), paramType)
		}
	}

	a.analyzeFunctionBlock(fn.Body)

	// Perform flow control analysis
	flowAnalyzer := NewFlowAnalyzer(a)
	flowAnalyzer.AnalyzeFunction(fn)

	// Restore previous scope
	a.symbols = previousScope
}

func (a *Analyzer) analyzeFunctionBlockItem(item ast.FunctionBlockItem) {
	switch node := item.(type) {
	case *ast.ExprStmt:
		a.analyzeExpression(node.Expr)
	case *ast.LetStmt:
		a.analyzeLetStatement(node)
	case *ast.ReturnStmt:
		if node.Value != nil {
			a.analyzeExpression(node.Value)
		}
	case *ast.RequireStmt:
		// Require can have multiple arguments
		for _, arg := range node.Args {
			a.analyzeExpression(arg)
		}
	case *ast.AssignStmt:
		a.analyzeAssignStatement(node)
	case *ast.IfStmt:
		// Analyze all branches to catch errors like immutable assignments or
		// undefined variables that only occur in conditional paths
		a.analyzeIfStatement(node)
	}
}

func (a *Analyzer) validateDirectFunctionCall(functionName string, call *ast.CallExpr) {
	// Check if function is imported or locally defined
	isImported := a.context.IsImportedFunction(functionName)
	_, isLocalFunction := a.localFunctions[functionName]

	if !isImported && !isLocalFunction {
		a.addUndefinedFunctionError(functionName, call.NodePos())
		return
	}

	// Get function definition for parameter validation
	var funcDef *stdlib.FunctionDefinition
	if isImported {
		funcDef = a.context.GetFunctionDefinition(functionName)
	}
	// Note: For local functions, we'd need to extract parameter info from AST
	// TODO: Implement local function signature extraction if needed

	if isImported && funcDef == nil {
		a.addError(fmt.Sprintf("function '%s' definition not found", functionName), call.NodePos())
		return
	}

	// Validate parameter count (only for imported functions with known signatures)
	if isImported && funcDef != nil {
		if len(call.Args) != len(funcDef.Parameters) {
			a.addCompilerError(errors.InvalidArguments(functionName, len(funcDef.Parameters), len(call.Args), call.NodePos()))
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
	} else {
		// For local functions, just analyze arguments without strict validation
		for _, arg := range call.Args {
			a.analyzeExpression(arg)
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
		fullName := fmt.Sprintf("%s::%s", moduleName, functionName)
		a.addCompilerError(errors.InvalidArguments(fullName, len(funcDef.Parameters), len(call.Args), call.NodePos()))
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

// isBuiltinFunction checks if a function is a built-in function
func (a *Analyzer) isBuiltinFunction(name string) bool {
	// Built-in functions that don't need to be explicitly imported
	builtins := map[string]bool{
		"require": true, // Built-in require macro
	}
	return builtins[name]
}

// isIndexableType checks if a type supports indexing operations
func (a *Analyzer) isIndexableType(typeRef *stdlib.TypeRef) bool {
	if typeRef == nil {
		return false
	}

	// Built-in indexable types (could be extended)
	switch typeRef.Name {
	case "Slots", "Vector", "Map":
		return true
	default:
		return false
	}
}

// validateAssignmentCompatibility checks if types are compatible for assignment
func (a *Analyzer) validateAssignmentCompatibility(leftType, rightType *stdlib.TypeRef, pos ast.Position) {
	if leftType == nil || rightType == nil {
		return
	}

	if !a.typesMatch(leftType, rightType) {
		if a.isNumericType(leftType) && a.isNumericType(rightType) {
			// Prevent silent data truncation that could cause overflow vulnerabilities in smart contracts
			if !a.canPromoteType(rightType, leftType) {
				a.addError(fmt.Sprintf("cannot assign %s to %s: potential precision loss",
					a.typeToString(rightType), a.typeToString(leftType)), pos)
			}
		} else {
			// Type safety prevents runtime errors and unexpected behavior in blockchain execution
			a.addError(fmt.Sprintf("cannot assign %s to %s: incompatible types",
				a.typeToString(rightType), a.typeToString(leftType)), pos)
		}
	}
}

// canPromoteType checks if source type can be promoted to target type
func (a *Analyzer) canPromoteType(source, target *stdlib.TypeRef) bool {
	typeOrder := map[string]int{
		"U8": 1, "U16": 2, "U32": 3, "U64": 4, "U128": 5, "U256": 6,
	}

	sourceOrder, sourceExists := typeOrder[source.Name]
	targetOrder, targetExists := typeOrder[target.Name]

	if !sourceExists || !targetExists {
		return false
	}

	// Can promote to same or wider type
	return sourceOrder <= targetOrder
}

// validateStructLiteralFields checks that struct literal fields match the struct definition
func (a *Analyzer) validateStructLiteralFields(structName string, fields []ast.StructLiteralField, pos ast.Position) {
	structDef := a.context.GetUserDefinedType(structName)
	if structDef == nil {
		return // Already reported as unknown type
	}

	// Build map of provided fields
	providedFields := make(map[string]bool)
	for _, field := range fields {
		fieldName := field.Name.Value
		if providedFields[fieldName] {
			a.addError(fmt.Sprintf("duplicate field '%s' in struct literal", fieldName), field.NodePos())
			continue
		}
		providedFields[fieldName] = true

		// Validate field exists in struct definition
		if !a.structHasField(structDef, fieldName) {
			a.addError(fmt.Sprintf("struct '%s' has no field '%s'", structName, fieldName), field.NodePos())
		}
	}

	// Check for missing required fields (basic check)
	for _, item := range structDef.Items {
		if field, ok := item.(*ast.StructField); ok {
			fieldName := field.Name.Value
			if !providedFields[fieldName] {
				a.addError(fmt.Sprintf("missing field '%s' in struct literal for '%s'", fieldName, structName), pos)
			}
		}
	}
}

// structHasField checks if a struct definition contains a specific field
func (a *Analyzer) structHasField(structDef *ast.Struct, fieldName string) bool {
	for _, item := range structDef.Items {
		if field, ok := item.(*ast.StructField); ok {
			if field.Name.Value == fieldName {
				return true
			}
		}
	}
	return false
}

// validateLiteralValue checks literal value format and bounds
func (a *Analyzer) validateLiteralValue(value string, pos ast.Position) {
	// Basic literal validation - could be improved with more specific checks
	if len(value) == 0 {
		a.addError("empty literal value", pos)
		return
	}

	// TODO: Add specific validation for different literal types:
	// - Numeric bounds checking
	// - String escape sequence validation
	// - Address format validation
}
