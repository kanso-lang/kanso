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

	// TODO: Complete call path analysis for storage access validation
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
	// TODO: Implement local function signature extraction for parameter validation

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
