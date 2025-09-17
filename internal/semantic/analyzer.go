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

// StorageAccess represents a direct storage access found in the code
type StorageAccess struct {
	StructName string // e.g. "State"
	FieldName  string // e.g. "balances"
	AccessType string // "read" or "write"
	Position   ast.Position
}

// FunctionCallGraph tracks function calls and storage accesses for call path analysis
type FunctionCallGraph struct {
	// DirectStorageAccesses maps function names to the storage they directly access
	DirectStorageAccesses map[string][]StorageAccess
	// FunctionCalls maps function names to the local functions they call
	FunctionCalls map[string][]string
	// RequiredReads/Writes maps function names to storage they need (including transitive)
	RequiredReads  map[string]map[string]bool
	RequiredWrites map[string]map[string]bool
}

type Analyzer struct {
	contract         *ast.Contract
	errors           []errors.CompilerError   // All errors with suggestions and proper formatting
	symbols          *SymbolTable             // Tracks variable/function scoping within contract
	context          *ContextRegistry         // Manages imports and standard library integration
	localFunctions   map[string]*ast.Function // Tracks functions defined in this contract
	callGraph        *FunctionCallGraph       // Tracks function calls and storage access for validation
	currentFunction  string                   // Name of the function currently being analyzed
	existingUseStmts []*ast.Use               // Tracks existing use statements for smart import suggestions
}

func NewAnalyzer() *Analyzer {
	return &Analyzer{
		errors:         make([]errors.CompilerError, 0),
		context:        NewContextRegistry(),
		localFunctions: make(map[string]*ast.Function),
		callGraph: &FunctionCallGraph{
			DirectStorageAccesses: make(map[string][]StorageAccess),
			FunctionCalls:         make(map[string][]string),
			RequiredReads:         make(map[string]map[string]bool),
			RequiredWrites:        make(map[string]map[string]bool),
		},
	}
}

type SemanticError struct {
	Message  string
	Position ast.Position
}

func (a *Analyzer) Analyze(contract *ast.Contract) []SemanticError {
	a.contract = contract
	a.errors = make([]errors.CompilerError, 0)
	a.localFunctions = make(map[string]*ast.Function) // Reset for each analysis
	a.symbols = NewSymbolTable(nil)                   // Root scope for contract-level declarations
	a.existingUseStmts = make([]*ast.Use, 0)          // Reset existing use statements

	// Reset call graph for each analysis
	a.callGraph = &FunctionCallGraph{
		DirectStorageAccesses: make(map[string][]StorageAccess),
		FunctionCalls:         make(map[string][]string),
		RequiredReads:         make(map[string]map[string]bool),
		RequiredWrites:        make(map[string]map[string]bool),
	}

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

// GetContext returns the context registry for IR generation
func (a *Analyzer) GetContext() *ContextRegistry {
	return a.context
}

// GetImportedFunction returns information about an imported function
func (a *Analyzer) GetImportedFunction(functionName string) *ImportedFunction {
	return a.context.GetImportedFunction(functionName)
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
			// Track existing use statements for smart import suggestions
			a.existingUseStmts = append(a.existingUseStmts, node)
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

	// Third pass: analyze call paths and validate reads/writes declarations
	// This must happen after all function bodies are analyzed so we have complete information
	a.performCallPathAnalysis(storageStructs)

	// Fourth pass: detect unused functions
	a.detectUnusedFunctions()

	// Note: Unused variable detection is performed per-function during analysis
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

	// Note: Call path analysis is now performed after all functions are processed
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

	// Initialize collections for this function
	functionName := fn.Name.Value
	a.callGraph.DirectStorageAccesses[functionName] = make([]StorageAccess, 0)
	a.callGraph.FunctionCalls[functionName] = make([]string, 0)

	// Set current function context for storage access and call tracking
	previousFunction := a.currentFunction
	a.currentFunction = functionName

	// Analyze function body and collect storage accesses and function calls
	a.analyzeFunctionBlock(fn.Body)

	// Restore previous function context
	a.currentFunction = previousFunction

	// Perform flow control analysis
	flowAnalyzer := NewFlowAnalyzer(a)
	flowAnalyzer.AnalyzeFunction(fn)

	// Detect unused variables in this function scope before restoring scope
	a.detectUnusedVariablesInScope(functionScope)

	// Restore previous scope
	a.symbols = previousScope
}

func (a *Analyzer) analyzeFunctionBlockItem(item ast.FunctionBlockItem) {
	switch node := item.(type) {
	case *ast.ExprStmt:
		a.analyzeExpression(node.Expr)
		// Check if we're ignoring a return value (could be a warning in the future)
		a.validateReturnValueUsage(node.Expr, false, nil) // false = value not required
	case *ast.LetStmt:
		a.analyzeLetStatement(node)
	case *ast.ReturnStmt:
		a.validateReturnStatement(node)
	case *ast.RequireStmt:
		// Require can have multiple arguments
		for _, arg := range node.Args {
			a.analyzeExpression(arg)
		}
	case *ast.AssignStmt:
		a.analyzeAssignStatement(node)
		// Get the expected type from the assignment target
		targetType := a.inferExpressionType(node.Target)
		a.validateReturnValueUsage(node.Value, true, targetType) // value is required for assignment with expected type
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
		a.addUndefinedFunctionErrorWithContext(functionName, call)
		return
	}

	// Track local function calls for call path analysis
	if isLocalFunction {
		a.addFunctionCall(functionName)
	}

	// Get function definition for parameter validation
	var funcDef *stdlib.FunctionDefinition
	if isImported {
		funcDef = a.context.GetFunctionDefinition(functionName)
	}
	// Get local function signature if available
	if isLocalFunction {
		funcDef = a.extractLocalFunctionSignature(functionName)
	}

	if isImported && funcDef == nil {
		a.addError(fmt.Sprintf("function '%s' definition not found", functionName), call.NodePos())
		return
	}

	// Validate parameter count for functions with known signatures
	if funcDef != nil {
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
		// For functions without known signatures, just analyze arguments without strict validation
		for _, arg := range call.Args {
			a.analyzeExpression(arg)
		}
	}
}

// extractLocalFunctionSignature creates a FunctionDefinition from a local function's AST
func (a *Analyzer) extractLocalFunctionSignature(functionName string) *stdlib.FunctionDefinition {
	function, exists := a.localFunctions[functionName]
	if !exists {
		return nil
	}

	// Extract parameters
	var parameters []stdlib.ParameterDefinition
	for _, param := range function.Params {
		paramType := a.convertASTTypeToTypeRef(param.Type)
		if paramType != nil {
			parameters = append(parameters, stdlib.ParameterDefinition{
				Name: param.Name.Value,
				Type: paramType,
			})
		}
	}

	// Extract return type
	var returnType *stdlib.TypeRef
	if function.Return != nil {
		returnType = a.convertASTTypeToTypeRef(function.Return)
	}

	return &stdlib.FunctionDefinition{
		Name:       functionName,
		Parameters: parameters,
		ReturnType: returnType,
		IsGeneric:  false, // Local functions are not generic for now
	}
}

// convertASTTypeToTypeRef converts an AST VariableType to a stdlib TypeRef
func (a *Analyzer) convertASTTypeToTypeRef(astType *ast.VariableType) *stdlib.TypeRef {
	if astType == nil {
		return nil
	}

	// Handle tuple types
	if len(astType.TupleElements) > 0 {
		// Convert each tuple element to a TypeRef
		elementTypes := make([]*stdlib.TypeRef, len(astType.TupleElements))
		for i, element := range astType.TupleElements {
			elementTypes[i] = a.convertASTTypeToTypeRef(element)
		}

		return &stdlib.TypeRef{
			Name:        "Tuple",
			IsGeneric:   false,
			GenericArgs: elementTypes,
		}
	}

	// Convert generic arguments if any
	var genericArgs []*stdlib.TypeRef
	for _, generic := range astType.Generics {
		if genericType := a.convertASTTypeToTypeRef(generic); genericType != nil {
			genericArgs = append(genericArgs, genericType)
		}
	}

	return &stdlib.TypeRef{
		Name:        astType.Name.Value,
		IsGeneric:   false, // AST types are concrete, not generic parameters
		GenericArgs: genericArgs,
	}
}

// validateTailExpression validates that tail expressions match the function's declared return type
func (a *Analyzer) validateTailExpression(tailExpr *ast.ExprStmt) {
	// Get the current function's return type
	var expectedReturnType *stdlib.TypeRef
	if a.currentFunction != "" {
		if fn, exists := a.localFunctions[a.currentFunction]; exists && fn != nil {
			if fn.Return != nil {
				expectedReturnType = a.convertASTTypeToTypeRef(fn.Return)
			}
		}
	}

	// Check if we're using a tail expression in a void function
	if expectedReturnType == nil {
		a.addError(fmt.Sprintf("void function '%s' cannot have a tail expression", a.currentFunction), tailExpr.NodePos())
		return
	}

	// Validate the tail expression type
	actualType := a.inferExpressionType(tailExpr.Expr)
	if actualType != nil && expectedReturnType != nil {
		if !a.areTypesCompatible(actualType, expectedReturnType) {
			a.addError(fmt.Sprintf("tail expression has type '%s' but function '%s' expects return type '%s'",
				a.typeToString(actualType), a.currentFunction, a.typeToString(expectedReturnType)), tailExpr.NodePos())
		}
	}

	// Also validate if it's a function call
	a.validateReturnValueUsage(tailExpr.Expr, true, expectedReturnType)
}

// validateReturnStatement validates that return statements match the function's declared return type
func (a *Analyzer) validateReturnStatement(returnStmt *ast.ReturnStmt) {
	// Get the current function's return type
	var expectedReturnType *stdlib.TypeRef
	if a.currentFunction != "" {
		if fn, exists := a.localFunctions[a.currentFunction]; exists && fn != nil {
			if fn.Return != nil {
				expectedReturnType = a.convertASTTypeToTypeRef(fn.Return)
			}
		}
	}

	if returnStmt.Value != nil {
		// Analyze the return expression
		a.analyzeExpression(returnStmt.Value)

		// Check if we're returning from a void function
		if expectedReturnType == nil {
			a.addError(fmt.Sprintf("void function '%s' cannot return a value", a.currentFunction), returnStmt.NodePos())
			return
		}

		// Validate the return value type
		actualType := a.inferExpressionType(returnStmt.Value)
		if actualType != nil && expectedReturnType != nil {
			if !a.areTypesCompatible(actualType, expectedReturnType) {
				a.addError(fmt.Sprintf("cannot return value of type '%s' from function with return type '%s'",
					a.typeToString(actualType), a.typeToString(expectedReturnType)), returnStmt.NodePos())
			}
		}

		// Also validate if it's a function call
		a.validateReturnValueUsage(returnStmt.Value, true, expectedReturnType)
	} else {
		// Empty return statement - check if function expects a return value
		if expectedReturnType != nil {
			a.addMissingReturnError(a.currentFunction, a.typeToString(expectedReturnType), returnStmt.NodePos())
		}
	}
}

// validateReturnValueUsage checks if function return values are properly handled
func (a *Analyzer) validateReturnValueUsage(expr ast.Expr, valueRequired bool, expectedType *stdlib.TypeRef) {
	// Only validate CallExpr nodes
	call, isCall := expr.(*ast.CallExpr)
	if !isCall {
		return
	}

	// Get the return type of the function
	returnType := a.inferCallExpressionType(call)

	if valueRequired && returnType == nil {
		// Error: Using void function where value is expected
		funcName := a.extractFunctionName(call)
		a.addVoidFunctionInExpressionError(funcName, call.NodePos())
		return
	}

	// If we have both a return type and an expected type, validate compatibility
	if valueRequired && returnType != nil && expectedType != nil {
		if !a.areTypesCompatible(returnType, expectedType) {
			funcName := a.extractFunctionName(call)
			returnTypeName := "void"
			if returnType != nil {
				returnTypeName = a.typeToString(returnType)
			}
			expectedTypeName := "void"
			if expectedType != nil {
				expectedTypeName = a.typeToString(expectedType)
			}
			a.addError(fmt.Sprintf("function '%s' returns '%s' but expected '%s'",
				funcName, returnTypeName, expectedTypeName), call.NodePos())
		}
	}

	// For now, we don't warn about ignored return values, but this could be added as a warning
	// if !valueRequired && returnType != nil {
	//     funcName := a.extractFunctionName(call)
	//     a.addWarning(fmt.Sprintf("ignoring return value of function '%s'", funcName), call.NodePos())
	// }
}

// extractFunctionName gets the function name from a CallExpr for error messages
func (a *Analyzer) extractFunctionName(call *ast.CallExpr) string {
	switch callee := call.Callee.(type) {
	case *ast.IdentExpr:
		return callee.Name
	case *ast.CalleePath:
		if len(callee.Parts) == 1 {
			return callee.Parts[0].Value
		} else if len(callee.Parts) == 2 {
			return fmt.Sprintf("%s::%s", callee.Parts[0].Value, callee.Parts[1].Value)
		}
	}
	return "unknown function"
}

// areTypesCompatible checks if two types are compatible for assignment
func (a *Analyzer) areTypesCompatible(actual, expected *stdlib.TypeRef) bool {
	if actual == nil || expected == nil {
		return false
	}

	// Exact match
	if actual.Name == expected.Name {
		// For generic types, check generic arguments too
		if len(actual.GenericArgs) != len(expected.GenericArgs) {
			return false
		}
		for i, actualArg := range actual.GenericArgs {
			if !a.areTypesCompatible(actualArg, expected.GenericArgs[i]) {
				return false
			}
		}
		return true
	}

	// Type promotion rules (e.g., smaller numeric types can be promoted to larger ones)
	return a.canPromoteType(actual, expected)
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
		a.addModuleNotImportedError(moduleName, call.NodePos())
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
	builtins := map[string]bool{
		"require": true,
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
				a.addTypeMismatchError(a.typeToString(leftType), a.typeToString(rightType), pos)
			}
		} else {
			// Type safety prevents runtime errors and unexpected behavior in blockchain execution
			a.addTypeMismatchError(a.typeToString(leftType), a.typeToString(rightType), pos)
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
			a.addDuplicateFieldError(fieldName, structName, field.NodePos())
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
				a.addMissingFieldError(fieldName, structName, pos)
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
	if len(value) == 0 {
		a.addError("empty literal value", pos)
		return
	}

	// Determine literal type and validate accordingly
	// Check address first since it can start with '0' like numbers
	switch {
	case value == "true" || value == "false":
		// Boolean literals are always valid
		return
	case a.isAddressLiteral(value):
		a.validateAddressLiteral(value, pos)
	case a.isStringLiteral(value):
		a.validateStringLiteral(value, pos)
	case a.isNumericLiteral(value):
		a.validateNumericLiteral(value, pos)
	default:
		// For other literals (like unquoted strings), perform basic validation
		// This is more permissive to handle cases where the parser stores
		// string content without quotes
		a.validateGenericLiteral(value, pos)
	}
}

// isStringLiteral checks if a value is a string literal (quoted)
func (a *Analyzer) isStringLiteral(value string) bool {
	return len(value) >= 2 && value[0] == '"' && value[len(value)-1] == '"'
}

// isAddressLiteral checks if a value looks like an Ethereum address
func (a *Analyzer) isAddressLiteral(value string) bool {
	// Ethereum address: special case 0x0 or exactly 42 characters (0x + 40 hex chars)
	// We need to be precise here to distinguish from hex numeric literals like 0x1, 0xFF, etc.
	return value == "0x0" || (len(value) == 42 && value[:2] == "0x")
}

// validateNumericLiteral validates numeric literal format and bounds
func (a *Analyzer) validateNumericLiteral(value string, pos ast.Position) {
	// Handle hexadecimal literals (0x prefix)
	if len(value) >= 2 && value[:2] == "0x" {
		a.validateHexNumericLiteral(value, pos)
		return
	}

	// Handle decimal literals
	// Check for invalid numeric formats (leading zeros not allowed in decimal)
	if len(value) > 1 && value[0] == '0' && value[1] >= '0' && value[1] <= '9' {
		a.addError(fmt.Sprintf("numeric literal '%s' has leading zeros (not allowed)", value), pos)
		return
	}

	// Check for extremely large numbers that might cause parsing issues
	if len(value) > 80 { // U256 max is ~78 digits
		a.addError(fmt.Sprintf("numeric literal '%s' is too large (exceeds maximum length)", value), pos)
		return
	}

	// Validate all characters are digits for decimal literals
	for i, r := range value {
		if r < '0' || r > '9' {
			a.addError(fmt.Sprintf("numeric literal '%s' contains invalid character '%c' at position %d", value, r, i), pos)
			return
		}
	}
}

// validateHexNumericLiteral validates hexadecimal numeric literals like 0x1, 0xFF, 0x2A
func (a *Analyzer) validateHexNumericLiteral(value string, pos ast.Position) {
	if len(value) < 3 { // Must be at least "0x" + one hex digit
		a.addError(fmt.Sprintf("hex literal '%s' is too short (must have at least one hex digit after 0x)", value), pos)
		return
	}

	// Check for extremely large hex numbers
	if len(value) > 66 { // 0x + 64 hex digits = 256 bits max
		a.addError(fmt.Sprintf("hex literal '%s' is too large (exceeds maximum length for U256)", value), pos)
		return
	}

	// Validate hex digits after 0x
	hexPart := value[2:]
	for i, r := range hexPart {
		if !a.isHexDigit(byte(r)) {
			a.addError(fmt.Sprintf("hex literal '%s' contains invalid hex character '%c' at position %d", value, r, i+2), pos)
			return
		}
	}
}

// validateStringLiteral validates string literal format and escape sequences
func (a *Analyzer) validateStringLiteral(value string, pos ast.Position) {
	if len(value) < 2 {
		a.addError("string literal too short (missing quotes)", pos)
		return
	}

	// Remove surrounding quotes for content validation
	content := value[1 : len(value)-1]

	// Validate escape sequences
	for i := 0; i < len(content); i++ {
		if content[i] == '\\' {
			if i == len(content)-1 {
				a.addError("string literal ends with incomplete escape sequence", pos)
				return
			}

			// Check valid escape sequences
			nextChar := content[i+1]
			switch nextChar {
			case 'n', 't', 'r', '\\', '"', '\'', '0':
				// Valid escape sequences
				i++ // Skip next character
			case 'x':
				// Hex escape sequence: \xHH
				if i+3 >= len(content) {
					a.addError("incomplete hex escape sequence in string literal", pos)
					return
				}
				hex1, hex2 := content[i+2], content[i+3]
				if !a.isHexDigit(hex1) || !a.isHexDigit(hex2) {
					a.addError(fmt.Sprintf("invalid hex escape sequence '\\x%c%c' in string literal", hex1, hex2), pos)
					return
				}
				i += 3 // Skip \xHH
			case 'u':
				// Unicode escape sequence: \uHHHH
				if i+5 >= len(content) {
					a.addError("incomplete unicode escape sequence in string literal", pos)
					return
				}
				for j := i + 2; j < i+6; j++ {
					if !a.isHexDigit(content[j]) {
						a.addError(fmt.Sprintf("invalid unicode escape sequence in string literal"), pos)
						return
					}
				}
				i += 5 // Skip \uHHHH
			default:
				a.addError(fmt.Sprintf("invalid escape sequence '\\%c' in string literal", nextChar), pos)
				return
			}
		}
	}
}

// validateAddressLiteral validates Ethereum address format
func (a *Analyzer) validateAddressLiteral(value string, pos ast.Position) {
	if value == "0x0" {
		return // Valid zero address
	}

	if len(value) != 42 {
		a.addError(fmt.Sprintf("address literal '%s' must be exactly 42 characters (0x + 40 hex digits)", value), pos)
		return
	}

	if value[:2] != "0x" {
		a.addError(fmt.Sprintf("address literal '%s' must start with '0x'", value), pos)
		return
	}

	// Validate hex characters
	hexPart := value[2:]
	for i, r := range hexPart {
		if !a.isHexDigit(byte(r)) {
			a.addError(fmt.Sprintf("address literal '%s' contains invalid hex character '%c' at position %d", value, r, i+2), pos)
			return
		}
	}

	// Note: We could add checksum validation here if needed, but it's not strictly required
	// for basic syntax validation
}

// validateGenericLiteral provides basic validation for other literal types
func (a *Analyzer) validateGenericLiteral(value string, pos ast.Position) {
	// This is a permissive fallback for literals that don't fit other categories
	// Perform basic safety checks without being overly restrictive

	// Check for extremely long literals that might cause memory issues
	if len(value) > 1000 { // Reasonable limit for most use cases
		a.addError(fmt.Sprintf("literal '%s' is too long (exceeds maximum length)", value), pos)
		return
	}

	// Check for control characters that might be problematic
	for i, r := range value {
		if r < 32 && r != '\t' && r != '\n' && r != '\r' { // Allow basic whitespace
			a.addError(fmt.Sprintf("literal contains invalid control character at position %d", i), pos)
			return
		}
	}

	// If it passes basic checks, allow it
	// This handles cases like unquoted strings, identifiers used as literals, etc.
}

// isHexDigit checks if a byte represents a valid hexadecimal digit
func (a *Analyzer) isHexDigit(b byte) bool {
	return (b >= '0' && b <= '9') || (b >= 'a' && b <= 'f') || (b >= 'A' && b <= 'F')
}

// performCallPathAnalysis analyzes the complete call graph and validates reads/writes declarations
func (a *Analyzer) performCallPathAnalysis(storageStructs map[string]bool) {
	// Step 1: Compute transitive closure of storage requirements
	a.computeTransitiveStorageRequirements()

	// Step 2: Validate that all functions declare their required reads/writes
	a.validateStorageDeclarations()
}

// computeTransitiveStorageRequirements propagates storage access requirements through the call graph
func (a *Analyzer) computeTransitiveStorageRequirements() {
	// Initialize direct requirements
	for funcName, accesses := range a.callGraph.DirectStorageAccesses {
		a.callGraph.RequiredReads[funcName] = make(map[string]bool)
		a.callGraph.RequiredWrites[funcName] = make(map[string]bool)

		// Add direct storage accesses
		for _, access := range accesses {
			if access.AccessType == "read" {
				a.callGraph.RequiredReads[funcName][access.StructName] = true
			} else if access.AccessType == "write" {
				a.callGraph.RequiredWrites[funcName][access.StructName] = true
				// Writes imply reads in EVM context
				a.callGraph.RequiredReads[funcName][access.StructName] = true
			}
		}
	}

	// Propagate requirements through call graph using fixed-point iteration
	changed := true
	for changed {
		changed = false
		for callerFunc, calledFuncs := range a.callGraph.FunctionCalls {
			for _, calledFunc := range calledFuncs {
				// Propagate reads from called function to caller
				if calledReads, exists := a.callGraph.RequiredReads[calledFunc]; exists {
					for structName := range calledReads {
						if !a.callGraph.RequiredReads[callerFunc][structName] {
							a.callGraph.RequiredReads[callerFunc][structName] = true
							changed = true
						}
					}
				}

				// Propagate writes from called function to caller
				if calledWrites, exists := a.callGraph.RequiredWrites[calledFunc]; exists {
					for structName := range calledWrites {
						if !a.callGraph.RequiredWrites[callerFunc][structName] {
							a.callGraph.RequiredWrites[callerFunc][structName] = true
							changed = true
						}
						// Writes also imply reads
						if !a.callGraph.RequiredReads[callerFunc][structName] {
							a.callGraph.RequiredReads[callerFunc][structName] = true
							changed = true
						}
					}
				}
			}
		}
	}
}

// validateStorageDeclarations ensures all functions declare the storage they access
func (a *Analyzer) validateStorageDeclarations() {
	for funcName, fn := range a.localFunctions {
		// Get declared reads and writes
		declaredReads := make(map[string]bool)
		declaredWrites := make(map[string]bool)

		for _, read := range fn.Reads {
			declaredReads[read.Value] = true
		}

		for _, write := range fn.Writes {
			declaredWrites[write.Value] = true
			// Writes imply reads, so don't require separate read declaration
			declaredReads[write.Value] = true
		}

		// Check required reads
		if requiredReads, exists := a.callGraph.RequiredReads[funcName]; exists {
			for structName := range requiredReads {
				if !declaredReads[structName] {
					a.addStorageAccessError(funcName, structName, false, fn.NodePos())
				}
			}
		}

		// Check required writes
		if requiredWrites, exists := a.callGraph.RequiredWrites[funcName]; exists {
			for structName := range requiredWrites {
				if !declaredWrites[structName] {
					a.addStorageAccessError(funcName, structName, true, fn.NodePos())
				}
			}
		}
	}
}

// addStorageAccess records a storage access for the current function being analyzed
func (a *Analyzer) addStorageAccess(structName, fieldName, accessType string, pos ast.Position) {
	// Find the current function being analyzed
	currentFunc := a.getCurrentFunctionName()
	if currentFunc == "" {
		return // Not in a function context
	}

	access := StorageAccess{
		StructName: structName,
		FieldName:  fieldName,
		AccessType: accessType,
		Position:   pos,
	}

	a.callGraph.DirectStorageAccesses[currentFunc] = append(
		a.callGraph.DirectStorageAccesses[currentFunc], access)
}

// addFunctionCall records a function call for the current function being analyzed
func (a *Analyzer) addFunctionCall(calledFunc string) {
	// Find the current function being analyzed
	currentFunc := a.getCurrentFunctionName()
	if currentFunc == "" {
		return // Not in a function context
	}

	// Check if it's a local function call
	if _, isLocal := a.localFunctions[calledFunc]; isLocal {
		// Avoid duplicates
		for _, existing := range a.callGraph.FunctionCalls[currentFunc] {
			if existing == calledFunc {
				return
			}
		}
		a.callGraph.FunctionCalls[currentFunc] = append(
			a.callGraph.FunctionCalls[currentFunc], calledFunc)
	}
}

// getCurrentFunctionName returns the name of the function currently being analyzed
func (a *Analyzer) getCurrentFunctionName() string {
	return a.currentFunction
}

// detectUnusedFunctions identifies functions that are defined but never called
func (a *Analyzer) detectUnusedFunctions() {
	// Create set of all called functions
	calledFunctions := make(map[string]bool)

	// Collect all function calls from the call graph
	for _, calledFuncs := range a.callGraph.FunctionCalls {
		for _, calledFunc := range calledFuncs {
			calledFunctions[calledFunc] = true
		}
	}

	// Check each local function
	for funcName, fn := range a.localFunctions {
		if fn == nil {
			continue
		}

		// Skip entry points that can be called externally
		if a.isFunctionEntryPoint(fn) {
			continue
		}

		// Report unused function
		if !calledFunctions[funcName] {
			a.addError(fmt.Sprintf("function '%s' is defined but never used", funcName), fn.NodePos())
		}
	}
}

// isFunctionEntryPoint checks if a function is an entry point (external or constructor)
func (a *Analyzer) isFunctionEntryPoint(fn *ast.Function) bool {
	// External functions can be called from outside the contract
	if fn.External {
		return true
	}

	// Constructor functions are called during deployment
	if fn.Attribute != nil && fn.Attribute.Name == "create" {
		return true
	}

	return false
}

func (a *Analyzer) detectUnusedVariablesInScope(scope *SymbolTable) {
	if scope == nil {
		return
	}

	// Check all variables in current scope (excluding parameters)
	for _, symbol := range scope.symbols {
		if symbol.Kind == SymbolVariable {
			// Check for unused variables
			if !symbol.Used {
				a.addError(fmt.Sprintf("variable '%s' is declared but never used", symbol.Name), symbol.Position)
			}

			// Check for problematic mutable variables
			if symbol.Mutable {
				if !symbol.Modified {
					// Mutable variable that's never modified
					a.addError(fmt.Sprintf("variable '%s' is declared as mutable but never modified", symbol.Name), symbol.Position)
				} else if symbol.Modified && !symbol.ReadAfterModify {
					// Mutable variable that's modified but never read after modification
					// Use the position of the last modification, not the declaration
					errorPos := symbol.LastModifyPos
					if errorPos.Line == 0 { // Fallback to declaration position if LastModifyPos not set
						errorPos = symbol.Position
					}
					a.addError(fmt.Sprintf("variable '%s' is modified but the new value is never used", symbol.Name), errorPos)
				}
			}
		}
	}
}
