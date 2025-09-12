package semantic

import (
	"fmt"
	"kanso/internal/ast"
	"kanso/internal/errors"
	"kanso/internal/stdlib"
	"math/big"
	"strconv"
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
			a.validateWritesReferences(fn.Writes, fn.NodePos())

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
			a.validateWritesReferences(fn.Writes, fn.NodePos())
		}
		if len(fn.Reads) > 0 {
			a.validateReadsReferences(fn.Reads, fn.NodePos())
		}
	}
}

func (a *Analyzer) validateWritesReferences(writes []ast.Ident, pos ast.Position) {
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

func (a *Analyzer) validateReadsReferences(reads []ast.Ident, pos ast.Position) {
	for _, structRef := range reads {
		structName := structRef.Value
		structType := a.context.GetUserDefinedType(structName)

		// Reads clauses enable gas optimization by declaring upfront which storage will be accessed.
		// Only storage structs contain persistent state worth reading - events are write-only logs
		// and regular structs don't persist across transactions
		if structType == nil || structType.Attribute == nil || structType.Attribute.Name != "storage" {
			a.addCompilerError(errors.InvalidReadsWrites(fmt.Sprintf("reads clause references non-storage struct: %s", structName), structRef.NodePos()))
		}
	}
}

func (a *Analyzer) validateFunctionReadsWrites(fn *ast.Function, storageStructs map[string]bool) {
	// Reads/writes validation enables gas optimization by declaring upfront which
	// state the function accesses, and prevents accidental state access patterns
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

		// Writing to storage requires reading it first (e.g., to update a map entry),
		// so declaring both reads() and writes() for the same struct is redundant
		// and suggests developer confusion about the access model
		if readStructs[write.Value] {
			a.addError("conflicting reads and writes clause for struct (write implies read): "+write.Value, write.NodePos())
		}
	}
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

func (a *Analyzer) analyzeExpression(expr ast.Expr) {
	if expr == nil {
		return
	}

	switch node := expr.(type) {
	case *ast.CallExpr:
		a.analyzeCallExpression(node)
	case *ast.FieldAccessExpr:
		a.analyzeExpression(node.Target)
		// Validate field access for semantic correctness
		a.analyzeFieldAccess(node)
	case *ast.IndexExpr:
		a.analyzeIndexExpression(node)
	case *ast.StructLiteralExpr:
		a.analyzeStructLiteralExpression(node)
	case *ast.ParenExpr:
		a.analyzeExpression(node.Value)
	case *ast.BinaryExpr:
		a.analyzeBinaryExpression(node)
	case *ast.UnaryExpr:
		a.analyzeUnaryExpression(node)
	case *ast.IdentExpr:
		a.analyzeIdentExpression(node)
	case *ast.LiteralExpr:
		a.analyzeLiteralExpression(node)
	case *ast.TupleExpr:
		a.analyzeTupleExpression(node)
		// Other expression types are already handled by type inference
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
	// For now, we skip parameter validation for local functions

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

// validateArgumentType validates that an argument expression matches the expected parameter type
func (a *Analyzer) validateArgumentType(arg ast.Expr, expectedType *stdlib.TypeRef, pos ast.Position) bool {
	// Get the inferred type of the argument expression with contextual hint
	argType := a.inferExpressionTypeWithContext(arg, expectedType)
	if argType == nil {
		// Cannot infer type - for now, allow it (could be improved later)
		return true
	}

	// Check if types match
	if !a.typesMatch(argType, expectedType) {
		// Allow numeric type promotion for compatibility
		if a.isNumericType(argType) && a.isNumericType(expectedType) {
			if a.canPromoteType(argType, expectedType) {
				return true // Allow promotion
			}
		}
		a.addError(fmt.Sprintf("argument type %s does not match expected type %s",
			a.typeToString(argType), a.typeToString(expectedType)), pos)
		return false
	}

	return true
}

// inferExpressionType performs comprehensive type inference for complex expressions
// It handles nested expressions, type coercion, and provides error recovery
func (a *Analyzer) inferExpressionType(expr ast.Expr) *stdlib.TypeRef {
	if expr == nil {
		return nil
	}

	switch node := expr.(type) {
	case *ast.LiteralExpr:
		return a.inferLiteralType(node.Value)
	case *ast.IdentExpr:
		return a.inferIdentifierType(node)
	case *ast.CallExpr:
		return a.inferCallExpressionType(node)
	case *ast.FieldAccessExpr:
		return a.inferFieldAccessType(node)
	case *ast.IndexExpr:
		return a.inferIndexExpressionType(node)
	case *ast.BinaryExpr:
		return a.inferBinaryExpressionType(node)
	case *ast.UnaryExpr:
		return a.inferUnaryExpressionType(node)
	case *ast.ParenExpr:
		return a.inferExpressionType(node.Value)
	case *ast.StructLiteralExpr:
		return a.inferStructLiteralType(node)
	case *ast.TupleExpr:
		return a.inferTupleExpressionType(node)
	default:
		// Unknown expression type - return nil for graceful degradation
		return nil
	}
}

// inferIdentifierType handles type inference for identifier expressions
func (a *Analyzer) inferIdentifierType(node *ast.IdentExpr) *stdlib.TypeRef {
	if node.Name == "true" || node.Name == "false" {
		return stdlib.BoolType()
	}

	// Check variables first (parameters, local variables) for most specific type info
	if symbol := a.symbols.Lookup(node.Name); symbol != nil {
		return symbol.Type
	}

	// Check user-defined types (structs) to enable State.field syntax
	if a.context.IsUserDefinedType(node.Name) {
		return &stdlib.TypeRef{Name: node.Name, IsGeneric: false}
	}

	// Check imported function return types
	if funcDef := a.context.GetFunctionDefinition(node.Name); funcDef != nil {
		return funcDef.ReturnType
	}

	return nil
}

// inferLiteralType provides enhanced literal type inference with better numeric type detection
func (a *Analyzer) inferLiteralType(value string) *stdlib.TypeRef {
	if value == "true" || value == "false" {
		return stdlib.BoolType()
	}
	// Enhanced address detection for various formats
	if value == "0x0" || (len(value) >= 2 && value[:2] == "0x" && len(value) == 42) {
		return stdlib.AddressType()
	}

	// Numeric literal inference with size-aware defaults
	if len(value) > 0 && (value[0] >= '0' && value[0] <= '9') {
		// Choose the smallest type for immutable context, U256 for mutable context
		return a.inferNumericLiteralType(value, ast.Position{})
	}

	// String literals (quoted)
	if len(value) >= 2 && value[0] == '"' && value[len(value)-1] == '"' {
		return &stdlib.TypeRef{Name: "String", IsGeneric: false}
	}

	// Default fallback for unknown literals
	// For EVM compatibility, default to U256 as it's the native word size
	return stdlib.U256Type()
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

func (a *Analyzer) analyzeLetStatement(letStmt *ast.LetStmt) {
	if a.checkVariableShadowing(letStmt) {
		return
	}

	a.checkMutabilityShadowing(letStmt)

	varType := a.determineVariableType(letStmt)
	if varType == nil {
		return // Error already reported
	}

	a.symbols.DefineVariable(letStmt.Name.Value, letStmt, letStmt.NodePos(), varType, letStmt.Mut)
}

func (a *Analyzer) checkVariableShadowing(letStmt *ast.LetStmt) bool {
	// Variable shadowing is forbidden to prevent subtle bugs where developers
	// accidentally reuse names and operate on the wrong variable
	if existing := a.symbols.LookupLocal(letStmt.Name.Value); existing != nil {
		a.addError(fmt.Sprintf("variable '%s' is already declared in this scope", letStmt.Name.Value), letStmt.NodePos())
		return true
	}
	return false
}

func (a *Analyzer) determineVariableType(letStmt *ast.LetStmt) *stdlib.TypeRef {
	// Handle explicit type annotations
	if letStmt.Type != nil {
		varType := a.resolveVariableType(letStmt.Type)
		if varType == nil {
			a.addError(fmt.Sprintf("unknown type '%s'", letStmt.Type.String()), letStmt.NodePos())
			return nil
		}

		if letStmt.Expr != nil {
			a.analyzeExpression(letStmt.Expr)
			a.validateExplicitTypeAssignment(letStmt, varType)
			return varType
		}

		// Explicit type but no expression - check if uninitialized is allowed
		if !letStmt.Mut {
			a.addError(fmt.Sprintf("immutable variable '%s' must be initialized at declaration", letStmt.Name.Value), letStmt.NodePos())
			return nil
		}
		return varType
	}

	// Handle uninitialized variables without explicit type
	if letStmt.Expr == nil {
		return a.handleUninitializedVariable(letStmt)
	}

	// Handle initialized variables with type inference
	return a.inferVariableType(letStmt)
}

func (a *Analyzer) handleUninitializedVariable(letStmt *ast.LetStmt) *stdlib.TypeRef {
	// Immutable variables must be initialized because their value can never change
	if !letStmt.Mut {
		a.addError(fmt.Sprintf("immutable variable '%s' must be initialized at declaration", letStmt.Name.Value), letStmt.NodePos())
		return nil
	}

	// Default to U256 for untyped mutable variables because:
	// 1. U256 is the EVM's native word size - no gas penalty for storage
	// 2. We cannot predict future assignments, so we use the most permissive type
	// 3. This matches Solidity's uint default and developer expectations
	return stdlib.U256Type()
}

func (a *Analyzer) inferVariableType(letStmt *ast.LetStmt) *stdlib.TypeRef {
	a.analyzeExpression(letStmt.Expr)

	// Type inference depends on mutability to optimize for different use cases:
	// - Immutable: smallest type for gas efficiency (value never changes)
	// - Mutable: U256 to accommodate any future assignment
	varType := a.inferExpressionTypeForVariable(letStmt.Expr, letStmt.Mut)

	if varType == nil {
		// Fallback for complex expressions where primary inference fails
		varType = a.attemptTypeInferenceRecovery(letStmt.Expr)
	}

	return varType
}

// validateExplicitTypeAssignment checks if the assigned value is compatible with the declared type
func (a *Analyzer) validateExplicitTypeAssignment(letStmt *ast.LetStmt, declaredType *stdlib.TypeRef) {
	if letStmt.Expr == nil || declaredType == nil {
		return
	}

	// For numeric literals, validate the value is within the type's range
	if litExpr, ok := letStmt.Expr.(*ast.LiteralExpr); ok && a.isNumericLiteral(litExpr.Value) {
		a.validateNumericLiteralRange(litExpr.Value, declaredType, litExpr.NodePos())
		return
	}

	// For non-literal expressions, infer the type and check compatibility
	inferredType := a.inferExpressionType(letStmt.Expr)
	if inferredType == nil {
		return // Error already reported during type inference
	}

	// Check type compatibility
	if !a.isTypeCompatible(inferredType, declaredType) {
		a.addError(fmt.Sprintf("cannot assign value of type '%s' to variable of type '%s'",
			inferredType.Name, declaredType.Name), letStmt.NodePos())
	}
}

// validateNumericLiteralRange checks if a numeric literal value fits within the declared type's range
func (a *Analyzer) validateNumericLiteralRange(value string, declaredType *stdlib.TypeRef, pos ast.Position) {
	// Parse the numeric value using big.Int for full range support
	bigNum := new(big.Int)
	if _, ok := bigNum.SetString(value, 10); !ok {
		a.addError(fmt.Sprintf("invalid numeric literal '%s'", value), pos)
		return
	}

	// Check if the value fits within the declared type's range
	var maxValue *big.Int
	var typeName string

	switch declaredType.Name {
	case "U8":
		maxValue = big.NewInt(255) // 2^8 - 1
		typeName = "U8"
	case "U16":
		maxValue = big.NewInt(65535) // 2^16 - 1
		typeName = "U16"
	case "U32":
		maxValue = big.NewInt(4294967295) // 2^32 - 1
		typeName = "U32"
	case "U64":
		maxValue = new(big.Int)
		maxValue.SetString("18446744073709551615", 10) // 2^64 - 1
		typeName = "U64"
	case "U128":
		maxValue = new(big.Int)
		maxValue.SetString("340282366920938463463374607431768211455", 10) // 2^128 - 1
		typeName = "U128"
	case "U256":
		maxValue = new(big.Int)
		maxValue.SetString("115792089237316195423570985008687907853269984665640564039457584007913129639935", 10) // 2^256 - 1
		typeName = "U256"
	default:
		// Non-numeric type or unknown type - no range validation needed
		return
	}

	// Check if the value is negative (not allowed for unsigned types)
	if bigNum.Sign() < 0 {
		a.addError(fmt.Sprintf("negative value '%s' cannot be assigned to unsigned type '%s'", value, typeName), pos)
		return
	}

	// Check if the value exceeds the maximum for the declared type
	if bigNum.Cmp(maxValue) > 0 {
		// Find the smallest type that would fit this value
		suggestedType := a.inferMinimalTypeForValue(value)
		if suggestedType != nil {
			a.addError(fmt.Sprintf("value '%s' exceeds maximum for type '%s' (max: %s), consider using '%s' instead",
				value, typeName, maxValue.String(), suggestedType.Name), pos)
		} else {
			a.addError(fmt.Sprintf("value '%s' exceeds maximum for type '%s' (max: %s)",
				value, typeName, maxValue.String()), pos)
		}
	}
}

// inferMinimalTypeForValue finds the smallest unsigned integer type that can hold the given value
func (a *Analyzer) inferMinimalTypeForValue(value string) *stdlib.TypeRef {
	return a.inferNumericLiteralType(value, ast.Position{})
}

// isTypeCompatible checks if two types are compatible for assignment
func (a *Analyzer) isTypeCompatible(from, to *stdlib.TypeRef) bool {
	if from == nil || to == nil {
		return false
	}

	// Exact type match
	if from.Name == to.Name {
		return true
	}

	// For now, we require exact type matches for explicit declarations
	// In the future, we could implement numeric type promotion here
	return false
}

func (a *Analyzer) analyzeAssignStatement(assignStmt *ast.AssignStmt) {
	a.analyzeExpression(assignStmt.Value)
	a.validateAssignmentTarget(assignStmt.Target, assignStmt.NodePos())

	if identExpr, ok := assignStmt.Target.(*ast.IdentExpr); ok {
		a.validateVariableAssignment(identExpr, assignStmt.NodePos())
	} else {
		// Complex targets (field access, indexing) need full validation
		a.analyzeExpression(assignStmt.Target)
	}
}

func (a *Analyzer) validateVariableAssignment(identExpr *ast.IdentExpr, pos ast.Position) {
	symbol := a.symbols.Lookup(identExpr.Name)
	if symbol == nil {
		a.addUndefinedVariableError(identExpr.Name, pos)
		return
	}

	// Immutability prevents accidental state changes that could break contract invariants
	if symbol.Kind == SymbolVariable && !symbol.Mutable {
		a.addImmutableVariableAssignmentError(identExpr.Name, pos)
	}
}

func (a *Analyzer) validateAssignmentTarget(target ast.Expr, pos ast.Position) {
	if a.isValidAssignmentTarget(target) {
		return
	}

	errorMsg := a.getInvalidAssignmentMessage(target)
	a.addCompilerError(errors.InvalidAssignment(errorMsg, pos))
}

func (a *Analyzer) isValidAssignmentTarget(target ast.Expr) bool {
	switch target.(type) {
	case *ast.IdentExpr, *ast.FieldAccessExpr, *ast.IndexExpr:
		return true
	default:
		return false
	}
}

func (a *Analyzer) getInvalidAssignmentMessage(target ast.Expr) string {
	switch target.(type) {
	case *ast.CallExpr:
		return "cannot assign to function call result"
	case *ast.BinaryExpr:
		return "cannot assign to binary expression"
	case *ast.UnaryExpr:
		return "cannot assign to unary expression"
	case *ast.LiteralExpr:
		return "cannot assign to literal value"
	default:
		return "invalid assignment target"
	}
}

// analyzeIfStatement performs semantic analysis on conditional statements.
//
// 1. Ensures all variable references in conditions are valid and in scope
// 2. Validates that assignments within conditional blocks respect mutability rules
// 3. Enables detection of unreachable code and missing return paths
// 4. Maintains symbol table consistency across branching execution paths
//
// The recursive analysis of nested blocks ensures that even deeply nested
// conditional logic (common in complex access control) is fully validated.
func (a *Analyzer) analyzeIfStatement(ifStmt *ast.IfStmt) {
	// Validate condition expression - must be semantically valid and type-checked
	// Common condition errors: undefined variables, type mismatches, invalid operations
	a.analyzeExpression(ifStmt.Condition)

	// Process then block with full semantic analysis
	// This catches mutability violations, undefined variables, and type errors
	// that occur within the conditional branch
	a.analyzeFunctionBlock(&ifStmt.ThenBlock)

	// Process optional else block with same rigor
	// Maintaining consistency between branches is crucial for predictable contract behavior
	if ifStmt.ElseBlock != nil {
		a.analyzeFunctionBlock(ifStmt.ElseBlock)
	}
}

// analyzeFunctionBlock provides centralized block analysis to ensure consistent
// semantic validation across all block types (functions, if statements, loops, etc.)
func (a *Analyzer) analyzeFunctionBlock(block *ast.FunctionBlock) {
	// Analyze all statements in the block for semantic correctness
	for _, item := range block.Items {
		a.analyzeFunctionBlockItem(item)
	}

	// Handle tail expressions (Rust-style implicit returns)
	// These are semantically significant as they can affect return type checking
	if block.TailExpr != nil {
		a.analyzeExpression(block.TailExpr.Expr)
	}
}

func (a *Analyzer) analyzeFieldAccess(fieldExpr *ast.FieldAccessExpr) *stdlib.TypeRef {
	targetType := a.inferExpressionType(fieldExpr.Target)

	if targetType == nil {
		return nil // Cannot validate without knowing target type
	}

	// Field access is only valid on struct types
	structDef := a.context.GetUserDefinedType(targetType.Name)
	if structDef == nil {
		a.addError(fmt.Sprintf("type '%s' is not a struct", targetType.Name), fieldExpr.NodePos())
		return nil
	}

	return a.validateStructField(structDef, fieldExpr.Field, fieldExpr.NodePos())
}

// inferFieldAccessType handles type inference for field access without validation
// This is used by the type inference system to avoid duplicate error reporting
func (a *Analyzer) inferFieldAccessType(fieldExpr *ast.FieldAccessExpr) *stdlib.TypeRef {
	targetType := a.inferExpressionType(fieldExpr.Target)

	if targetType == nil {
		return nil // Cannot infer without knowing target type
	}

	// Get struct definition for type inference
	structDef := a.context.GetUserDefinedType(targetType.Name)
	if structDef == nil {
		return nil // Not a struct type
	}

	// Find field type without generating errors
	for _, item := range structDef.Items {
		if field, ok := item.(*ast.StructField); ok {
			if field.Name.Value == fieldExpr.Field {
				return a.resolveVariableType(field.VariableType)
			}
		}
	}

	// Field not found, return nil
	return nil
}

func (a *Analyzer) validateStructField(structNode *ast.Struct, fieldName string, pos ast.Position) *stdlib.TypeRef {
	for _, item := range structNode.Items {
		if field, ok := item.(*ast.StructField); ok {
			if field.Name.Value == fieldName {
				return a.resolveVariableType(field.VariableType)
			}
		}
	}

	a.addFieldNotFoundError(structNode.Name.Value, fieldName, pos)
	return nil
}

func (a *Analyzer) resolveVariableType(varType *ast.VariableType) *stdlib.TypeRef {
	if varType == nil {
		return nil
	}

	typeName := varType.Name.Value

	// Map AST type names to standard library type references
	switch typeName {
	case "U8":
		return stdlib.U8Type()
	case "U16":
		return stdlib.U16Type()
	case "U32":
		return stdlib.U32Type()
	case "U64":
		return stdlib.U64Type()
	case "U128":
		return stdlib.U128Type()
	case "U256":
		return stdlib.U256Type()
	case "Bool":
		return stdlib.BoolType()
	case "Address":
		return stdlib.AddressType()
	case "String":
		return &stdlib.TypeRef{Name: "String", IsGeneric: false}
	case "Slots":
		// Handle Slots generic type - need to process generic arguments
		return a.resolveGenericType(varType)
	}

	// Support user-defined struct types
	if a.context.IsUserDefinedType(typeName) {
		return &stdlib.TypeRef{Name: typeName, IsGeneric: false}
	}

	return nil // Unknown type
}

// resolveGenericType handles resolution of generic types like Slots<Address, U256>
func (a *Analyzer) resolveGenericType(varType *ast.VariableType) *stdlib.TypeRef {
	typeName := varType.Name.Value

	var genericArgs []*stdlib.TypeRef
	for _, genericType := range varType.Generics {
		resolved := a.resolveVariableType(genericType)
		if resolved == nil {
			return nil
		}
		genericArgs = append(genericArgs, resolved)
	}

	return &stdlib.TypeRef{
		Name:        typeName,
		IsGeneric:   len(genericArgs) > 0,
		GenericArgs: genericArgs,
	}
}

func (a *Analyzer) inferBinaryExpressionType(binExpr *ast.BinaryExpr) *stdlib.TypeRef {
	leftType := a.inferExpressionType(binExpr.Left)
	rightType := a.inferExpressionType(binExpr.Right)

	if leftType == nil || rightType == nil {
		return nil // Cannot infer without operand types
	}

	switch binExpr.Op {
	case "+", "-", "*", "/", "%":
		if a.isNumericType(leftType) && a.isNumericType(rightType) {
			return a.promoteNumericType(leftType, rightType)
		}
		a.addError(fmt.Sprintf("invalid operation: %s %s %s",
			a.typeToString(leftType), binExpr.Op, a.typeToString(rightType)), binExpr.NodePos())
		return nil

	case "==", "!=", "<", "<=", ">", ">=":
		// Allow comparison between same types or between numeric types
		if a.typesMatch(leftType, rightType) || (a.isNumericType(leftType) && a.isNumericType(rightType)) {
			return stdlib.BoolType()
		}
		a.addError(fmt.Sprintf("invalid comparison: %s %s %s",
			a.typeToString(leftType), binExpr.Op, a.typeToString(rightType)), binExpr.NodePos())
		return stdlib.BoolType() // Return Bool for error recovery

	case "&&", "||":
		if a.isBoolType(leftType) && a.isBoolType(rightType) {
			return stdlib.BoolType()
		}
		a.addError(fmt.Sprintf("invalid logical operation: %s %s %s",
			a.typeToString(leftType), binExpr.Op, a.typeToString(rightType)), binExpr.NodePos())
		return stdlib.BoolType()

	default:
		return nil
	}
}

func (a *Analyzer) inferUnaryExpressionType(unExpr *ast.UnaryExpr) *stdlib.TypeRef {
	operandType := a.inferExpressionType(unExpr.Value)

	if operandType == nil {
		return nil
	}

	switch unExpr.Op {
	case "-", "+":
		if a.isNumericType(operandType) {
			return operandType // Preserve original numeric type
		}
		a.addError(fmt.Sprintf("invalid unary operation: %s%s",
			unExpr.Op, a.typeToString(operandType)), unExpr.NodePos())
		return nil

	case "!":
		if a.isBoolType(operandType) {
			return stdlib.BoolType()
		}
		a.addError(fmt.Sprintf("invalid logical negation: !%s",
			a.typeToString(operandType)), unExpr.NodePos())
		return stdlib.BoolType() // Return Bool for error recovery

	default:
		return nil
	}
}

func (a *Analyzer) isNumericType(typeRef *stdlib.TypeRef) bool {
	if typeRef == nil {
		return false
	}

	switch typeRef.Name {
	case "U8", "U16", "U32", "U64", "U128", "U256":
		return true
	default:
		return false
	}
}

func (a *Analyzer) isBoolType(typeRef *stdlib.TypeRef) bool {
	return typeRef != nil && typeRef.Name == "Bool"
}

func (a *Analyzer) promoteNumericType(left, right *stdlib.TypeRef) *stdlib.TypeRef {
	// Use blockchain-appropriate promotion: wider types accommodate more precision
	// and prevent overflow in financial calculations
	typeOrder := map[string]int{
		"U8": 1, "U16": 2, "U32": 3, "U64": 4, "U128": 5, "U256": 6,
	}

	leftOrder, leftExists := typeOrder[left.Name]
	rightOrder, rightExists := typeOrder[right.Name]

	if !leftExists || !rightExists {
		return left // Fallback to preserve existing type
	}

	if rightOrder > leftOrder {
		return right
	}
	return left
}

// analyzeIndexExpression validates array/mapping index operations
func (a *Analyzer) analyzeIndexExpression(indexExpr *ast.IndexExpr) {
	a.analyzeExpression(indexExpr.Target)
	a.analyzeExpression(indexExpr.Index)

	// Validate that target supports indexing
	targetType := a.inferExpressionType(indexExpr.Target)
	if targetType != nil {
		if !a.isIndexableType(targetType) {
			a.addError(fmt.Sprintf("type '%s' does not support indexing", a.typeToString(targetType)), indexExpr.NodePos())
		}
	}

	// For now, we allow any index type - could be improved for specific container types
}

// analyzeStructLiteralExpression validates struct literal field assignments
func (a *Analyzer) analyzeStructLiteralExpression(structExpr *ast.StructLiteralExpr) {
	// Analyze all field values
	for _, field := range structExpr.Fields {
		a.analyzeExpression(field.Value)
	}

	// Validate that the struct type exists
	if structExpr.Type != nil && len(structExpr.Type.Parts) > 0 {
		structName := structExpr.Type.Parts[0].Value
		if !a.context.IsUserDefinedType(structName) {
			a.addError(fmt.Sprintf("unknown struct type '%s'", structName), structExpr.NodePos())
			return
		}

		// Validate field assignments match struct definition
		a.validateStructLiteralFields(structName, structExpr.Fields, structExpr.NodePos())
	}
}

// analyzeBinaryExpression provides binary operation validation
func (a *Analyzer) analyzeBinaryExpression(binExpr *ast.BinaryExpr) {
	a.analyzeExpression(binExpr.Left)
	a.analyzeExpression(binExpr.Right)

	// The type inference already handles most validation, but we can add
	// additional semantic checks here if needed
	leftType := a.inferExpressionType(binExpr.Left)
	rightType := a.inferExpressionType(binExpr.Right)

	// Validation for assignment operations
	if binExpr.Op == "=" || binExpr.Op == "+=" || binExpr.Op == "-=" ||
		binExpr.Op == "*=" || binExpr.Op == "/=" || binExpr.Op == "%=" {
		a.validateAssignmentCompatibility(leftType, rightType, binExpr.NodePos())
	}
}

// analyzeUnaryExpression provides unary operation validation
func (a *Analyzer) analyzeUnaryExpression(unExpr *ast.UnaryExpr) {
	a.analyzeExpression(unExpr.Value)

	// The type inference already handles validation
	// Additional semantic checks could be added here
}

// analyzeIdentExpression validates identifier references
func (a *Analyzer) analyzeIdentExpression(identExpr *ast.IdentExpr) {
	// Check if identifier is defined (variable, function, type, etc.)
	if identExpr.Name != "true" && identExpr.Name != "false" {
		if symbol := a.symbols.Lookup(identExpr.Name); symbol == nil {
			if !a.context.IsUserDefinedType(identExpr.Name) &&
				!a.context.IsImportedFunction(identExpr.Name) &&
				!a.isBuiltinFunction(identExpr.Name) {
				a.addUndefinedVariableError(identExpr.Name, identExpr.NodePos())
			}
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

// analyzeLiteralExpression validates literal values
func (a *Analyzer) analyzeLiteralExpression(litExpr *ast.LiteralExpr) {
	// Validate literal format and bounds
	a.validateLiteralValue(litExpr.Value, litExpr.NodePos())
}

// analyzeTupleExpression validates tuple expressions
func (a *Analyzer) analyzeTupleExpression(tupleExpr *ast.TupleExpr) {
	for _, element := range tupleExpr.Elements {
		a.analyzeExpression(element)
	}
}

// isIndexableType checks if a type supports indexing operations
func (a *Analyzer) isIndexableType(typeRef *stdlib.TypeRef) bool {
	if typeRef == nil {
		return false
	}

	// Built-in indexable types (could be extended)
	switch typeRef.Name {
	case "Slots", "Table", "Array", "Map":
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

func (a *Analyzer) addError(message string, pos ast.Position) {
	// Fallback for simple errors that don't need specialized handling with suggestions
	err := errors.NewSemanticError(errors.ErrorGenericSemantic, message, pos).Build()
	a.errors = append(a.errors, err)
}

func (a *Analyzer) addCompilerError(err errors.CompilerError) {
	a.errors = append(a.errors, err)
}

func (a *Analyzer) addUndefinedVariableError(name string, pos ast.Position) {
	// Provide typo suggestions to reduce developer frustration with common mistakes
	similar := a.findSimilarVariables(name)
	err := errors.UndefinedVariable(name, pos, similar)
	a.addCompilerError(err)
}

func (a *Analyzer) addUndefinedFunctionError(name string, pos ast.Position) {
	// Help developers discover available standard library functions and fix typos
	similar := a.findSimilarFunctions(name)
	imports := a.findPossibleImports(name)
	err := errors.UndefinedFunction(name, pos, similar, imports)
	a.addCompilerError(err)
}

func (a *Analyzer) addTypeMismatchError(expected, actual string, pos ast.Position) {
	err := errors.TypeMismatch(expected, actual, pos)
	a.addCompilerError(err)
}

func (a *Analyzer) addFieldNotFoundError(structName, fieldName string, pos ast.Position) {
	// Show available fields to help with autocompletion and typo detection
	availableFields := a.getStructFields(structName)
	err := errors.FieldNotFound(structName, fieldName, pos, availableFields)
	a.addCompilerError(err)
}

func (a *Analyzer) addImmutableVariableAssignmentError(varName string, pos ast.Position) {
	// Provide specific help for making variables mutable
	err := errors.NewSemanticError(errors.ErrorInvalidAssignment,
		fmt.Sprintf("cannot assign to immutable variable '%s'", varName), pos).
		WithHelp(fmt.Sprintf("variable '%s' is declared as immutable", varName)).
		WithSuggestion(fmt.Sprintf("change 'let %s' to 'let mut %s' to make it mutable", varName, varName)).
		WithNote("only variables declared with 'let mut' can be reassigned").
		Build()
	a.addCompilerError(err)
}

// Helper methods for finding similar names and suggestions

func (a *Analyzer) findSimilarVariables(name string) []string {
	var similar []string

	// Check current scope and parent scopes
	for scope := a.symbols; scope != nil; scope = scope.parent {
		for varName := range scope.symbols {
			if levenshteinDistance(name, varName) <= 2 && len(varName) > 1 {
				similar = append(similar, varName)
			}
		}
	}

	return similar
}

func (a *Analyzer) findSimilarFunctions(name string) []string {
	var similar []string

	// Check local functions
	for funcName := range a.localFunctions {
		if levenshteinDistance(name, funcName) <= 2 && len(funcName) > 1 {
			similar = append(similar, funcName)
		}
	}

	// Check imported functions
	// This would need to be implemented in the context registry

	return similar
}

func (a *Analyzer) findPossibleImports(name string) []string {
	// This would check the standard library for functions with similar names
	// and suggest the appropriate import statements
	var imports []string

	// Example suggestions based on common function names
	switch name {
	case "send", "sender":
		imports = append(imports, "std::evm::{sender}")
	case "emit":
		imports = append(imports, "std::evm::{emit}")
	case "zero":
		imports = append(imports, "std::address::{zero}")
	}

	return imports
}

func (a *Analyzer) getStructFields(structName string) []string {
	var fields []string

	structDef := a.context.GetUserDefinedType(structName)
	if structDef != nil {
		for _, item := range structDef.Items {
			if field, ok := item.(*ast.StructField); ok {
				fields = append(fields, field.Name.Value)
			}
		}
	}

	return fields
}

// Simple Levenshtein distance for finding similar names
func levenshteinDistance(a, b string) int {
	if len(a) == 0 {
		return len(b)
	}
	if len(b) == 0 {
		return len(a)
	}

	if len(a) > len(b) {
		a, b = b, a
	}

	previous := make([]int, len(a)+1)
	for i := range previous {
		previous[i] = i
	}

	for i := 0; i < len(b); i++ {
		current := make([]int, len(a)+1)
		current[0] = i + 1

		for j := 0; j < len(a); j++ {
			cost := 0
			if a[j] != b[i] {
				cost = 1
			}
			current[j+1] = min3(
				current[j]+1,     // insertion
				previous[j+1]+1,  // deletion
				previous[j]+cost, // substitution
			)
		}
		previous = current
	}

	return previous[len(a)]
}

func min3(a, b, c int) int {
	if a < b {
		if a < c {
			return a
		}
		return c
	}
	if b < c {
		return b
	}
	return c
}

// inferNumericLiteralType chooses the smallest unsigned integer type that can hold a numeric literal.
func (a *Analyzer) inferNumericLiteralType(value string, pos ast.Position) *stdlib.TypeRef {
	// First attempt: parse as uint64 to handle common cases efficiently
	if num, err := strconv.ParseUint(value, 10, 64); err == nil {
		// Select the minimal type based on standard bit boundaries
		switch {
		case num <= 255:
			return stdlib.U8Type()
		case num <= 65535:
			return stdlib.U16Type()
		case num <= 4294967295:
			return stdlib.U32Type()
		default:
			return stdlib.U64Type()
		}
	}

	// Second attempt: handle values larger than uint64 using big.Int
	bigNum := new(big.Int)
	if _, ok := bigNum.SetString(value, 10); ok {
		// U128 max: 2^128 - 1
		u128Max := new(big.Int)
		u128Max.SetString("340282366920938463463374607431768211455", 10)

		if bigNum.Cmp(u128Max) <= 0 {
			return stdlib.U128Type()
		}

		// U256 max: 2^256 - 1
		u256Max := new(big.Int)
		u256Max.SetString("115792089237316195423570985008687907853269984665640564039457584007913129639935", 10)

		if bigNum.Cmp(u256Max) <= 0 {
			return stdlib.U256Type()
		}

		// Numeric literal exceeds U256 maximum - report error only if position is valid
		if pos.Line > 0 && pos.Column > 0 {
			a.addError(fmt.Sprintf("numeric literal '%s' exceeds maximum value for U256", value), pos)
		}
		return nil
	}

	// Invalid numeric literal format - report error only if position is valid
	if pos.Line > 0 && pos.Column > 0 {
		a.addError(fmt.Sprintf("invalid numeric literal '%s'", value), pos)
	}
	return nil
}

// inferIndexExpressionType handles type inference for array/mapping access
func (a *Analyzer) inferIndexExpressionType(node *ast.IndexExpr) *stdlib.TypeRef {
	targetType := a.inferExpressionType(node.Target)
	if targetType == nil {
		return nil
	}

	// Handle known indexable types
	switch targetType.Name {
	case "Slots":
		// Slots<K, V> returns V
		if len(targetType.GenericArgs) >= 2 {
			return targetType.GenericArgs[1]
		}
	case "Table", "Map":
		// Similar pattern for other container types
		if len(targetType.GenericArgs) >= 2 {
			return targetType.GenericArgs[1]
		}
	case "Array":
		// Array<T> returns T
		if len(targetType.GenericArgs) >= 1 {
			return targetType.GenericArgs[0]
		}
	}

	// For unknown indexable types, return nil
	return nil
}

// inferStructLiteralType handles type inference for struct literals
func (a *Analyzer) inferStructLiteralType(node *ast.StructLiteralExpr) *stdlib.TypeRef {
	if node.Type == nil || len(node.Type.Parts) == 0 {
		return nil
	}

	structName := node.Type.Parts[0].Value
	if a.context.IsUserDefinedType(structName) {
		return &stdlib.TypeRef{Name: structName, IsGeneric: false}
	}

	return nil
}

// inferTupleExpressionType handles type inference for tuple expressions
func (a *Analyzer) inferTupleExpressionType(node *ast.TupleExpr) *stdlib.TypeRef {
	// For now, we don't have a robust tuple type system
	// This could be enhanced to return a tuple type with element types
	return &stdlib.TypeRef{Name: "Tuple", IsGeneric: false}
}

// areComparableTypes checks if two types can be compared
func (a *Analyzer) areComparableTypes(left, right *stdlib.TypeRef) bool {
	if left == nil || right == nil {
		return left == right
	}

	// Same types are always comparable
	if a.typesMatch(left, right) {
		return true
	}

	// Numeric types are comparable with each other
	if a.isNumericType(left) && a.isNumericType(right) {
		return true
	}

	// Bool types are comparable with each other
	if a.isBoolType(left) && a.isBoolType(right) {
		return true
	}

	return false
}

// isStringType checks if a type is a string type
func (a *Analyzer) isStringType(typeRef *stdlib.TypeRef) bool {
	return typeRef != nil && typeRef.Name == "String"
}

// mutabilityAnalysis provides comprehensive mutability checking for let mut variables
func (a *Analyzer) checkMutabilityShadowing(letStmt *ast.LetStmt) {
	// Warn about shadowing with different mutability to prevent confusion
	// when a variable changes from mutable to immutable or vice versa
	varName := letStmt.Name.Value

	existing := a.symbols.Lookup(varName)
	if existing != nil && existing.Kind == SymbolVariable && existing.Mutable != letStmt.Mut {
		a.addError(fmt.Sprintf("variable '%s' shadows existing variable with different mutability", varName), letStmt.NodePos())
	}

	// Validate that mutable variables are used in contexts where mutability makes sense
	if letStmt.Mut {
		// Track mutable variable usage for analysis
		a.trackMutableVariableDeclaration(varName, letStmt.NodePos())
	}
}

// trackMutableVariableDeclaration tracks the declaration of mutable variables
func (a *Analyzer) trackMutableVariableDeclaration(varName string, pos ast.Position) {
	// This could be enhanced to track mutable variable usage patterns
	// and warn about unused mutability or unnecessary mutations
}

// attemptTypeInferenceRecovery tries to recover type information when initial inference fails
func (a *Analyzer) attemptTypeInferenceRecovery(expr ast.Expr) *stdlib.TypeRef {
	// This method provides fallback type inference for complex cases
	// where the primary inference might fail

	switch node := expr.(type) {
	case *ast.BinaryExpr:
		// Try to infer from context or operands
		leftType := a.inferExpressionType(node.Left)
		rightType := a.inferExpressionType(node.Right)

		if leftType != nil {
			return leftType
		}
		if rightType != nil {
			return rightType
		}

		// Default fallback based on operation
		switch node.Op {
		case "==", "!=", "<", "<=", ">", ">=", "&&", "||":
			return stdlib.BoolType()
		case "+", "-", "*", "/", "%":
			return stdlib.U256Type() // Safe default for EVM arithmetic
		}

	case *ast.CallExpr:
		// For unknown function calls, try to infer from context
		// This could be enhanced with more sophisticated heuristics
		return nil

	case *ast.LiteralExpr:
		// Re-attempt literal inference with more permissive rules
		return a.inferLiteralType(node.Value)
	}

	return nil
}

// inferExpressionTypeWithContext performs type inference with contextual hints for better accuracy
func (a *Analyzer) inferExpressionTypeWithContext(expr ast.Expr, expectedType *stdlib.TypeRef) *stdlib.TypeRef {
	if expr == nil {
		return nil
	}

	// For numeric literals, use contextual type when available
	if litExpr, ok := expr.(*ast.LiteralExpr); ok {
		if a.isNumericLiteral(litExpr.Value) && expectedType != nil && a.isNumericType(expectedType) {
			// Use the expected type for numeric literals in function calls
			return expectedType
		}
	}

	// For other expressions, fall back to regular type inference
	return a.inferExpressionType(expr)
}

// isNumericLiteral checks if a string represents a numeric literal
func (a *Analyzer) isNumericLiteral(value string) bool {
	if len(value) == 0 {
		return false
	}
	// Simple check: starts with digit
	return value[0] >= '0' && value[0] <= '9'
}

// inferExpressionTypeForVariable chooses types based on mutability to optimize gas costs
// while preventing runtime overflow errors. This distinction is critical because:
// - Immutable variables can be aggressively optimized (smallest possible storage)
// - Mutable variables need defensive typing (accommodate any future value)
func (a *Analyzer) inferExpressionTypeForVariable(expr ast.Expr, isMutable bool) *stdlib.TypeRef {
	if litExpr, ok := expr.(*ast.LiteralExpr); ok {
		if a.isNumericLiteral(litExpr.Value) {
			if isMutable {
				// U256 for mutable prevents overflow bugs when the variable is later
				// assigned larger values. This is a safety-first approach since we
				// cannot statically analyze all possible future assignments.
				return stdlib.U256Type()
			} else {
				// Smallest type for immutable saves gas on every storage operation.
				// Safe because immutable values cannot change after initialization.
				return a.inferNumericLiteralType(litExpr.Value, litExpr.NodePos())
			}
		}
	}

	// Non-literal expressions use standard type inference (function returns, etc.)
	return a.inferExpressionType(expr)
}
