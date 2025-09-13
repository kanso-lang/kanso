package semantic

import (
	"fmt"
	"kanso/internal/ast"
	"kanso/internal/errors"
	"kanso/internal/stdlib"
	"math/big"
)

func (a *Analyzer) analyzeLetStatement(letStmt *ast.LetStmt) {
	if a.checkVariableShadowing(letStmt) {
		return
	}

	a.checkMutabilityShadowing(letStmt)

	varType := a.determineVariableType(letStmt)
	if varType == nil {
		return // Error already reported
	}

	// Analyze the initialization expression if present
	if letStmt.Expr != nil {
		a.analyzeExpression(letStmt.Expr)
		// Validate return value usage with expected type
		a.validateReturnValueUsage(letStmt.Expr, true, varType) // value is required for assignment with expected type
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

	// Type inference depends on mutability to optimize for different use cases:
	// - Immutable: smallest type for gas efficiency (value never changes)
	// - Mutable: U256 to accommodate any future assignment
	varType := a.inferExpressionTypeForVariable(letStmt.Expr, letStmt.Mut)

	if varType == nil {
		// Fallback for complex expressions where primary inference fails
		varType = a.attemptTypeInferenceRecovery(letStmt.Expr)
	}

	// Final fallback: if we still can't infer the type, use U256 to ensure the variable
	// is at least defined in the symbol table. This prevents "undefined variable" errors
	// for complex expressions where type inference fails.
	if varType == nil {
		varType = stdlib.U256Type()
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
	// Determine base and parse value for parsing
	base := 10
	parseValue := value
	if len(value) >= 2 && value[:2] == "0x" {
		base = 16
		parseValue = value[2:] // Remove 0x prefix for parsing
	}

	// Parse the numeric value using big.Int for full range support
	bigNum := new(big.Int)
	if _, ok := bigNum.SetString(parseValue, base); !ok {
		a.addError(fmt.Sprintf("invalid numeric literal '%s'", value), pos)
		return
	}

	// Check if the value fits within the declared type's range
	maxValue := a.getTypeMaxValue(declaredType.Name)
	if maxValue == nil {
		// Non-numeric type or unknown type - no range validation needed
		return
	}
	typeName := declaredType.Name

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

	// Type promotion for numeric types (e.g., U8 -> U256)
	return a.canPromoteType(from, to)
}

func (a *Analyzer) analyzeAssignStatement(assignStmt *ast.AssignStmt) {
	a.analyzeExpression(assignStmt.Value)
	a.validateAssignmentTarget(assignStmt.Target, assignStmt.NodePos())

	if identExpr, ok := assignStmt.Target.(*ast.IdentExpr); ok {
		a.validateVariableAssignment(identExpr, assignStmt.NodePos())
	} else {
		// Complex targets (field access, indexing) need full validation
		// Analyze in write context to track storage writes
		a.analyzeExpressionInWriteContext(assignStmt.Target)
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
		// Validate tail expression type against function return type
		a.validateTailExpression(block.TailExpr)
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

	// Track storage access if this is a storage struct
	a.trackStorageAccessIfNeeded(fieldExpr, structDef)

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

// mutabilityAnalysis provides comprehensive mutability checking for let mut variables
func (a *Analyzer) checkMutabilityShadowing(letStmt *ast.LetStmt) {
	// Warn about shadowing with different mutability to prevent confusion
	// when a variable changes from mutable to immutable or vice versa
	varName := letStmt.Name.Value

	existing := a.symbols.Lookup(varName)
	if existing != nil && existing.Kind == SymbolVariable && existing.Mutable != letStmt.Mut {
		a.addError(fmt.Sprintf("variable '%s' shadows existing variable with different mutability", varName), letStmt.NodePos())
	}

	// Note: Mutable variable usage tracking could be added here for optimization warnings
}

// trackStorageAccessIfNeeded records storage access if the field access is on a storage struct
func (a *Analyzer) trackStorageAccessIfNeeded(fieldExpr *ast.FieldAccessExpr, structDef *ast.Struct) {
	// Check if this is a storage struct
	if structDef.Attribute == nil || structDef.Attribute.Name != "storage" {
		return // Not a storage struct
	}

	// For now, assume all field accesses are reads by default
	// We'll track writes separately in assignment context
	a.addStorageAccess(structDef.Name.Value, fieldExpr.Field, "read", fieldExpr.NodePos())
}

// analyzeExpressionInWriteContext analyzes expressions that are being written to
// This is specifically for tracking storage writes in assignment contexts
func (a *Analyzer) analyzeExpressionInWriteContext(expr ast.Expr) {
	switch node := expr.(type) {
	case *ast.FieldAccessExpr:
		// Analyze the target first
		a.analyzeExpression(node.Target)

		// Track this as a write if it's a storage field
		targetType := a.inferExpressionType(node.Target)
		if targetType != nil {
			structDef := a.context.GetUserDefinedType(targetType.Name)
			if structDef != nil && structDef.Attribute != nil && structDef.Attribute.Name == "storage" {
				a.addStorageAccess(structDef.Name.Value, node.Field, "write", node.NodePos())
			}
		}

		// Also do normal field access validation
		a.analyzeFieldAccess(node)

	case *ast.IndexExpr:
		// For index expressions like State.field[key], we need to check the target
		a.analyzeExpressionInWriteContext(node.Target)
		a.analyzeExpression(node.Index)

	default:
		// For other expression types, fall back to normal analysis
		a.analyzeExpression(expr)
	}
}
