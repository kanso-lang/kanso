package semantic

import (
	"fmt"
	"kanso/internal/ast"
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
	errors         []SemanticError
	symbols        *SymbolTable             // Tracks variable/function scoping within contract
	context        *ContextRegistry         // Manages imports and standard library integration
	localFunctions map[string]*ast.Function // Tracks functions defined in this contract
}

type SemanticError struct {
	Message  string
	Position ast.Position
}

func NewAnalyzer() *Analyzer {
	return &Analyzer{
		errors:         make([]SemanticError, 0),
		context:        NewContextRegistry(),
		localFunctions: make(map[string]*ast.Function),
	}
}

func (a *Analyzer) Analyze(contract *ast.Contract) []SemanticError {
	a.contract = contract
	a.errors = make([]SemanticError, 0)
	a.localFunctions = make(map[string]*ast.Function) // Reset for each analysis
	a.symbols = NewSymbolTable(nil)                   // Root scope for contract-level declarations

	a.analyzeContract(contract)

	return a.errors
}

func (a *Analyzer) analyzeContract(contract *ast.Contract) {
	// Two-pass analysis is required because struct types must be known before
	// validating function reads/writes clauses that reference those types
	storageStructs := make(map[string]bool)

	// Include leading comments to handle license/doc comments in analysis
	allItems := make([]ast.ContractItem, 0, len(contract.LeadingComments)+len(contract.Items))
	allItems = append(allItems, contract.LeadingComments...)
	allItems = append(allItems, contract.Items...)

	// First pass: establish type context for imports, user-defined types, and local functions
	for _, item := range allItems {
		switch node := item.(type) {
		case *ast.Use:
			importErrors := a.context.ProcessUseStatement(node)
			for _, err := range importErrors {
				a.addError(err, node.NodePos())
			}
		case *ast.Struct:
			a.context.AddUserDefinedType(node.Name.Value, node)
			// Storage structs are the only ones that can be referenced in reads/writes
			if node.Attribute != nil && node.Attribute.Name == "storage" {
				storageStructs[node.Name.Value] = true
			}
		case *ast.Function:
			// Collect local functions for reference validation
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
				a.validateConstructor(node, storageStructs)
			}
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
	// Reads/writes validation enables gas optimization by declaring upfront which
	// state the function accesses, and prevents accidental state access patterns
	readStructs := make(map[string]bool)
	for _, read := range fn.Reads {
		if !storageStructs[read.Value] {
			a.addError("reads clause references non-storage struct: "+read.Value, read.NodePos())
			continue
		}

		if readStructs[read.Value] {
			a.addError("duplicate reads clause for struct: "+read.Value, read.NodePos())
		}
		readStructs[read.Value] = true
	}

	writeStructs := make(map[string]bool)
	for _, write := range fn.Writes {
		if !storageStructs[write.Value] {
			a.addError("writes clause references non-storage struct: "+write.Value, write.NodePos())
			continue
		}

		if writeStructs[write.Value] {
			a.addError("duplicate writes clause for struct: "+write.Value, write.NodePos())
		}
		writeStructs[write.Value] = true

		// Write implies read, so explicit read+write is redundant and potentially confusing
		if readStructs[write.Value] {
			a.addError("conflicting reads and writes clause for struct (write implies read): "+write.Value, write.NodePos())
		}
	}
}

func (a *Analyzer) validateConstructor(fn *ast.Function, storageStructs map[string]bool) {
	// Constructor constraints enforce blockchain deployment semantics where
	// constructors initialize state exactly once and cannot be called again
	if fn.Return != nil {
		a.addError("constructor functions cannot have a return type", fn.NodePos())
	}

	if len(fn.Writes) == 0 {
		a.addError("constructor functions must have a writes clause", fn.NodePos())
	} else {
		// Constructors must initialize at least one storage struct to be meaningful
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

	// Analyze all items in the function body
	for _, item := range fn.Body.Items {
		a.analyzeFunctionBlockItem(item)
	}

	// Analyze tail expression if present
	if fn.Body.TailExpr != nil {
		a.analyzeExpression(fn.Body.TailExpr.Expr)
	}

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
	// Check if function is imported or locally defined
	isImported := a.context.IsImportedFunction(functionName)
	_, isLocalFunction := a.localFunctions[functionName]

	if !isImported && !isLocalFunction {
		a.addError(fmt.Sprintf("function '%s' is not imported or defined", functionName), call.NodePos())
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

func (a *Analyzer) inferExpressionType(expr ast.Expr) *stdlib.TypeRef {
	switch node := expr.(type) {
	case *ast.LiteralExpr:
		return a.inferLiteralType(node.Value)
	case *ast.IdentExpr:
		if node.Name == "true" || node.Name == "false" {
			return stdlib.BoolType()
		}
		// Prioritize struct types to enable State.field syntax
		if a.context.IsUserDefinedType(node.Name) {
			return &stdlib.TypeRef{Name: node.Name, IsGeneric: false}
		}
		// Check variables (parameters, local variables)
		if symbol := a.symbols.Lookup(node.Name); symbol != nil {
			return symbol.Type
		}
		// Check imported functions
		if funcDef := a.context.GetFunctionDefinition(node.Name); funcDef != nil {
			return funcDef.ReturnType
		}
		return nil
	case *ast.CallExpr:
		return a.inferCallExpressionType(node)
	case *ast.FieldAccessExpr:
		return a.analyzeFieldAccess(node)
	case *ast.BinaryExpr:
		return a.inferBinaryExpressionType(node)
	case *ast.UnaryExpr:
		return a.inferUnaryExpressionType(node)
	case *ast.ParenExpr:
		return a.inferExpressionType(node.Value)
	default:
		return nil
	}
}

func (a *Analyzer) inferLiteralType(value string) *stdlib.TypeRef {
	if value == "true" || value == "false" {
		return stdlib.BoolType()
	}
	if value == "0x0" {
		return stdlib.AddressType()
	}
	// Default numeric literals to U64 as it matches most standard library
	// function signatures and avoids excessive type annotation requirements
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

func (a *Analyzer) analyzeLetStatement(letStmt *ast.LetStmt) {
	// Prevent variable shadowing within the same scope to avoid confusion
	if existing := a.symbols.LookupLocal(letStmt.Name.Value); existing != nil {
		a.addError(fmt.Sprintf("variable '%s' is already declared in this scope", letStmt.Name.Value), letStmt.NodePos())
		return
	}

	var varType *stdlib.TypeRef

	if letStmt.Expr != nil {
		a.analyzeExpression(letStmt.Expr)
		varType = a.inferExpressionType(letStmt.Expr)
	}

	// Register variable with mutability flag for assignment validation
	a.symbols.DefineVariable(letStmt.Name.Value, letStmt, letStmt.NodePos(), varType, letStmt.Mut)
}

func (a *Analyzer) analyzeAssignStatement(assignStmt *ast.AssignStmt) {
	a.analyzeExpression(assignStmt.Value)
	a.analyzeExpression(assignStmt.Target)

	// Enforce immutability constraints to prevent accidental modification
	if identExpr, ok := assignStmt.Target.(*ast.IdentExpr); ok {
		if symbol := a.symbols.Lookup(identExpr.Name); symbol != nil {
			if symbol.Kind == SymbolVariable && !symbol.Mutable {
				a.addError(fmt.Sprintf("cannot assign to immutable variable '%s'", identExpr.Name), assignStmt.NodePos())
			}
		} else {
			a.addError(fmt.Sprintf("undefined variable '%s'", identExpr.Name), assignStmt.NodePos())
		}
	}
	// Complex assignment targets (State.field, array[index]) handled by expression analysis
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

func (a *Analyzer) validateStructField(structNode *ast.Struct, fieldName string, pos ast.Position) *stdlib.TypeRef {
	for _, item := range structNode.Items {
		if field, ok := item.(*ast.StructField); ok {
			if field.Name.Value == fieldName {
				return a.resolveVariableType(field.VariableType)
			}
		}
	}

	a.addError(fmt.Sprintf("struct '%s' has no field '%s'", structNode.Name.Value, fieldName), pos)
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
	}

	// Support user-defined struct types
	if a.context.IsUserDefinedType(typeName) {
		return &stdlib.TypeRef{Name: typeName, IsGeneric: false}
	}

	return nil // Unknown type
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

func (a *Analyzer) addError(message string, pos ast.Position) {
	a.errors = append(a.errors, SemanticError{
		Message:  message,
		Position: pos,
	})
}
