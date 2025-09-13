package semantic

import (
	"fmt"
	"kanso/internal/ast"
	"kanso/internal/stdlib"
)

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

// inferCallExpressionType infers the return type of a function call
func (a *Analyzer) inferCallExpressionType(call *ast.CallExpr) *stdlib.TypeRef {
	switch callee := call.Callee.(type) {
	case *ast.IdentExpr:
		// Check local functions first
		if localFunc, exists := a.localFunctions[callee.Name]; exists && localFunc != nil {
			if localFunc.Return != nil {
				return a.convertASTTypeToTypeRef(localFunc.Return)
			}
			return nil // void function
		}
		// Then check imported functions
		if funcDef := a.context.GetFunctionDefinition(callee.Name); funcDef != nil {
			return funcDef.ReturnType
		}
	case *ast.CalleePath:
		if len(callee.Parts) == 1 {
			// Direct function call - check local first
			funcName := callee.Parts[0].Value
			if localFunc, exists := a.localFunctions[funcName]; exists && localFunc != nil {
				if localFunc.Return != nil {
					return a.convertASTTypeToTypeRef(localFunc.Return)
				}
				return nil // void function
			}
			// Then check imported
			if funcDef := a.context.GetFunctionDefinition(funcName); funcDef != nil {
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

	// Additional semantic checks can be added here if needed
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
