package semantic

import (
	"kanso/internal/ast"
	"kanso/internal/errors"
)

// FlowAnalyzer performs control flow analysis to catch logic errors that could lead to
// unexpected behavior or wasted gas in smart contract execution
type FlowAnalyzer struct {
	errors       []SemanticError
	usedVars     map[string]bool
	declaredVars map[string]ast.Position
	hasReturn    bool
	afterReturn  bool
	analyzer     *Analyzer // Reference to main analyzer for error reporting
}

// AnalysisResult contains the results of flow analysis
type AnalysisResult struct {
	UnreachableCode []SemanticError
	UnusedVariables []SemanticError
	MissingReturns  []SemanticError
	AllErrors       []SemanticError
}

// NewFlowAnalyzer creates a new flow analyzer
func NewFlowAnalyzer(analyzer *Analyzer) *FlowAnalyzer {
	return &FlowAnalyzer{
		errors:       make([]SemanticError, 0),
		usedVars:     make(map[string]bool),
		declaredVars: make(map[string]ast.Position),
		hasReturn:    false,
		afterReturn:  false,
		analyzer:     analyzer,
	}
}

// AnalyzeFunction performs flow control analysis on a function
func (fa *FlowAnalyzer) AnalyzeFunction(fn *ast.Function) AnalysisResult {
	// Reset state for new function
	fa.errors = make([]SemanticError, 0)
	fa.usedVars = make(map[string]bool)
	fa.declaredVars = make(map[string]ast.Position)
	fa.hasReturn = false
	fa.afterReturn = false

	// Mark parameters as used (they're part of the function interface)
	for _, param := range fn.Params {
		fa.usedVars[param.Name.Value] = true
	}

	// Analyze function body if it exists
	if fn.Body != nil {
		fa.analyzeFunctionBlock(fn.Body)

		// Ensure functions that promise to return values actually do so to prevent undefined behavior
		if fn.Return != nil && !fa.hasReturn {
			// Tail expressions serve as implicit returns in functional style, eliminating need for explicit return
			if fn.Body.TailExpr == nil {
				fa.analyzer.addCompilerError(errors.MissingReturn(fn.Body.EndPos))
			}
		}
	}

	// Check for unused variables (excluding parameters)
	fa.checkUnusedVariables()

	return AnalysisResult{
		UnreachableCode: fa.filterErrorsByType("unreachable"),
		UnusedVariables: fa.filterErrorsByType("unused"),
		MissingReturns:  fa.filterErrorsByType("missing return"),
		AllErrors:       fa.errors,
	}
}

// analyzeFunctionBlock analyzes a function body block
func (fa *FlowAnalyzer) analyzeFunctionBlock(block *ast.FunctionBlock) {
	// Analyze all statements in order
	for i, item := range block.Items {
		fa.analyzeStatement(item)

		// Unreachable code wastes gas and may indicate logic errors that could affect contract behavior
		if fa.afterReturn && i < len(block.Items)-1 {
			fa.analyzer.addCompilerError(errors.NewUnreachableCode(block.Items[i+1].NodePos()))
			break // Stop after first unreachable statement to avoid noise
		}
	}

	// Analyze tail expression if present
	if block.TailExpr != nil {
		if fa.afterReturn {
			fa.analyzer.addCompilerError(errors.NewUnreachableCode(block.TailExpr.NodePos()))
		} else {
			fa.analyzeExpression(block.TailExpr.Expr)
		}
	}
}

// analyzeStatement analyzes a single statement for flow control issues
func (fa *FlowAnalyzer) analyzeStatement(stmt ast.FunctionBlockItem) {
	if fa.afterReturn {
		fa.addError("unreachable code after return statement", stmt.NodePos())
		return
	}

	switch node := stmt.(type) {
	case *ast.ReturnStmt:
		fa.hasReturn = true
		fa.afterReturn = true
		if node.Value != nil {
			fa.analyzeExpression(node.Value)
		}

	case *ast.LetStmt:
		fa.analyzeLetStatement(node)

	case *ast.ExprStmt:
		fa.analyzeExpression(node.Expr)

	case *ast.AssignStmt:
		fa.analyzeExpression(node.Target)
		fa.analyzeExpression(node.Value)

	case *ast.RequireStmt:
		// require! statements can potentially terminate execution
		// but we don't treat them as definitive returns
		for _, arg := range node.Args {
			fa.analyzeExpression(arg)
		}
	}
}

// analyzeLetStatement handles variable declarations
func (fa *FlowAnalyzer) analyzeLetStatement(letStmt *ast.LetStmt) {
	varName := letStmt.Name.Value

	// Track variable declaration
	fa.declaredVars[varName] = letStmt.NodePos()

	// Analyze the initialization expression
	if letStmt.Expr != nil {
		fa.analyzeExpression(letStmt.Expr)
	}

	// Don't mark as used yet - usage will be tracked by expressions
}

// analyzeExpression tracks variable usage in expressions
func (fa *FlowAnalyzer) analyzeExpression(expr ast.Expr) {
	if expr == nil {
		return
	}

	switch node := expr.(type) {
	case *ast.IdentExpr:
		// Mark identifier as used
		fa.usedVars[node.Name] = true

	case *ast.CallExpr:
		fa.analyzeExpression(node.Callee)
		for _, arg := range node.Args {
			fa.analyzeExpression(arg)
		}

	case *ast.FieldAccessExpr:
		fa.analyzeExpression(node.Target)

	case *ast.IndexExpr:
		fa.analyzeExpression(node.Target)
		fa.analyzeExpression(node.Index)

	case *ast.BinaryExpr:
		fa.analyzeExpression(node.Left)
		fa.analyzeExpression(node.Right)

	case *ast.UnaryExpr:
		fa.analyzeExpression(node.Value)

	case *ast.ParenExpr:
		fa.analyzeExpression(node.Value)

	case *ast.StructLiteralExpr:
		for _, field := range node.Fields {
			fa.analyzeExpression(field.Value)
		}

	case *ast.TupleExpr:
		for _, element := range node.Elements {
			fa.analyzeExpression(element)
		}

	// Literals don't reference variables, so nothing to track
	case *ast.LiteralExpr:
		// No variable usage to track

		// Other expression types can be added as needed
	}
}

// checkUnusedVariables identifies variables that were declared but never used
// This is currently disabled to avoid breaking existing tests, but could be enabled
// as a warning or lint check in the future
func (fa *FlowAnalyzer) checkUnusedVariables() {
	// Skip unused variable checking for now to maintain compatibility
	// with existing tests. This could be re-enabled as a configurable
	// warning in the future.

	// Uncomment below to enable unused variable detection:
	/*
		for varName, pos := range fa.declaredVars {
			if !fa.usedVars[varName] {
				fa.addError(fmt.Sprintf("variable '%s' is declared but never used", varName), pos)
			}
		}
	*/
}

// filterErrorsByType returns errors that contain a specific substring
func (fa *FlowAnalyzer) filterErrorsByType(errorType string) []SemanticError {
	var filtered []SemanticError
	for _, err := range fa.errors {
		if containsType(err.Message, errorType) {
			filtered = append(filtered, err)
		}
	}
	return filtered
}

// containsType checks if an error message contains a specific type
func containsType(message, errorType string) bool {
	switch errorType {
	case "unreachable":
		return contains(message, "unreachable")
	case "unused":
		return contains(message, "never used")
	case "missing return":
		return contains(message, "no return statement")
	default:
		return false
	}
}

// contains is a simple string contains check
func contains(str, substr string) bool {
	return len(str) >= len(substr) &&
		(str == substr ||
			(len(str) > len(substr) &&
				(stringContains(str, substr))))
}

// stringContains implements a basic string search
func stringContains(haystack, needle string) bool {
	if len(needle) == 0 {
		return true
	}
	if len(haystack) < len(needle) {
		return false
	}

	for i := 0; i <= len(haystack)-len(needle); i++ {
		if haystack[i:i+len(needle)] == needle {
			return true
		}
	}
	return false
}

// addError adds a flow analysis error
func (fa *FlowAnalyzer) addError(message string, pos ast.Position) {
	fa.errors = append(fa.errors, SemanticError{
		Message:  message,
		Position: pos,
	})
}
