package semantic

import (
	"fmt"
	"kanso/internal/ast"
	"kanso/internal/stdlib"
	"math/big"
	"strconv"
)

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

	// Numeric literal inference with size-aware defaults (decimal and hex)
	if a.isNumericLiteral(value) {
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
		// Check numeric promotion before failing
		return a.isNumericPromotion(actual, expected)
	}

	// For tuple types, compare element types recursively
	if actual.Name == "Tuple" && expected.Name == "Tuple" {
		return a.tupleTypesMatch(actual, expected)
	}

	// For other generic types, compare generic arguments
	if len(actual.GenericArgs) != len(expected.GenericArgs) {
		return false
	}

	for i := 0; i < len(actual.GenericArgs); i++ {
		if !a.typesMatch(actual.GenericArgs[i], expected.GenericArgs[i]) {
			return false
		}
	}

	return true
}

// tupleTypesMatch checks if two tuple types are compatible
func (a *Analyzer) tupleTypesMatch(actual, expected *stdlib.TypeRef) bool {
	if len(actual.GenericArgs) != len(expected.GenericArgs) {
		return false
	}

	for i := 0; i < len(actual.GenericArgs); i++ {
		if !a.typesMatch(actual.GenericArgs[i], expected.GenericArgs[i]) {
			return false
		}
	}

	return true
}

// isNumericPromotion checks if actual can be promoted to expected
func (a *Analyzer) isNumericPromotion(actual, expected *stdlib.TypeRef) bool {
	if actual == nil || expected == nil {
		return false
	}

	// Define promotion hierarchy: U8 -> U16 -> U32 -> U64 -> U128 -> U256
	promotions := map[string][]string{
		"U8":   {"U16", "U32", "U64", "U128", "U256"},
		"U16":  {"U32", "U64", "U128", "U256"},
		"U32":  {"U64", "U128", "U256"},
		"U64":  {"U128", "U256"},
		"U128": {"U256"},
	}

	if validTargets, exists := promotions[actual.Name]; exists {
		for _, target := range validTargets {
			if target == expected.Name {
				return true
			}
		}
	}

	return false
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

	// Special case for tuple types - use parentheses syntax
	if typeRef.Name == "Tuple" {
		result := "("
		for i, arg := range typeRef.GenericArgs {
			if i > 0 {
				result += ", "
			}
			result += a.typeToString(arg)
		}
		result += ")"
		return result
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

// attemptTypeInferenceRecovery provides fallback type inference for complex expressions
func (a *Analyzer) attemptTypeInferenceRecovery(expr ast.Expr) *stdlib.TypeRef {

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
		// Use the same inference logic as the main type inference
		return a.inferCallExpressionType(node)

	case *ast.LiteralExpr:
		// Re-attempt literal inference with more permissive rules
		return a.inferLiteralType(node.Value)
	}

	return nil
}

// inferNumericLiteralType chooses the smallest unsigned integer type that can hold a numeric literal.
func (a *Analyzer) inferNumericLiteralType(value string, pos ast.Position) *stdlib.TypeRef {
	// Determine base: hexadecimal (0x prefix) or decimal
	base := 10
	parseValue := value
	if len(value) >= 2 && value[:2] == "0x" {
		base = 16
		parseValue = value[2:] // Remove 0x prefix for parsing
	}

	// First attempt: parse as uint64 to handle common cases efficiently
	if num, err := strconv.ParseUint(parseValue, base, 64); err == nil {
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
	if _, ok := bigNum.SetString(parseValue, base); ok {
		// Check U128 range
		if u128Max := a.getTypeMaxValue("U128"); u128Max != nil && bigNum.Cmp(u128Max) <= 0 {
			return stdlib.U128Type()
		}

		// Check U256 range
		if u256Max := a.getTypeMaxValue("U256"); u256Max != nil && bigNum.Cmp(u256Max) <= 0 {
			return stdlib.U256Type()
		}

		// Numeric literal exceeds U256 maximum - report error only if position is valid
		if pos.Line > 0 && pos.Column > 0 {
			a.addNumericOverflowError(value, "U256", "115792089237316195423570985008687907853269984665640564039457584007913129639935", "", pos)
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
	case "Map":
		// Similar pattern for other container types
		if len(targetType.GenericArgs) >= 2 {
			return targetType.GenericArgs[1]
		}
	case "Vector":
		// Vector<T> returns T
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

// inferTupleExpressionType handles type inference for tuple expressions
func (a *Analyzer) inferTupleExpressionType(node *ast.TupleExpr) *stdlib.TypeRef {
	if len(node.Elements) == 0 {
		// Empty tuple - rare but possible
		return &stdlib.TypeRef{Name: "Tuple", IsGeneric: false, GenericArgs: []*stdlib.TypeRef{}}
	}

	// Infer the type of each element
	elementTypes := make([]*stdlib.TypeRef, len(node.Elements))
	for i, element := range node.Elements {
		elementType := a.inferExpressionType(element)
		if elementType == nil {
			// If we can't infer an element type, try recovery inference
			elementType = a.attemptTypeInferenceRecovery(element)
		}
		if elementType == nil {
			// If we still can't infer the type, use a generic placeholder
			elementType = &stdlib.TypeRef{Name: "Unknown", IsGeneric: false}
		}
		elementTypes[i] = elementType
	}

	// Create a tuple type with element types as generic arguments
	return &stdlib.TypeRef{
		Name:        "Tuple",
		IsGeneric:   false,
		GenericArgs: elementTypes,
	}
}
