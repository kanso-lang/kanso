package semantic

import (
	"fmt"
	"kanso/internal/ast"
	"kanso/internal/errors"
)

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

func (a *Analyzer) addUninitializedImmutableError(varName string, pos ast.Position) {
	err := errors.NewSemanticError(errors.ErrorUninitializedVariable,
		fmt.Sprintf("immutable variable '%s' must be initialized at declaration", varName), pos).
		WithHelp("immutable variables cannot be assigned after declaration").
		WithSuggestion(fmt.Sprintf("initialize the variable: 'let %s = <value>'", varName)).
		WithNote("use 'let mut' if you need to assign the value later").
		Build()
	a.addCompilerError(err)
}

func (a *Analyzer) addInvalidOperationError(leftType, op, rightType string, pos ast.Position) {
	err := errors.NewSemanticError(errors.ErrorTypeMismatch,
		fmt.Sprintf("invalid operation: %s %s %s", leftType, op, rightType), pos).
		WithHelp(fmt.Sprintf("operator '%s' cannot be used between types '%s' and '%s'", op, leftType, rightType)).
		WithSuggestion("ensure both operands have compatible types").
		WithNote("numeric operations require numeric types, logical operations require Bool").
		Build()
	a.addCompilerError(err)
}

func (a *Analyzer) addMissingReturnError(funcName, returnType string, pos ast.Position) {
	err := errors.NewSemanticError(errors.ErrorMissingReturn,
		fmt.Sprintf("function '%s' must return a value of type '%s'", funcName, returnType), pos).
		WithHelp(fmt.Sprintf("function has return type '%s' but no value is returned", returnType)).
		WithSuggestion(fmt.Sprintf("add 'return <value>' or use a tail expression of type '%s'", returnType)).
		WithNote("functions with return types must return a value in all code paths").
		Build()
	a.addCompilerError(err)
}

func (a *Analyzer) addNumericOverflowError(value, typeName, maxValue string, suggestedType string, pos ast.Position) {
	if suggestedType != "" {
		err := errors.NewSemanticError(errors.ErrorNumericOverflow,
			fmt.Sprintf("value '%s' exceeds maximum for type '%s' (max: %s)", value, typeName, maxValue), pos).
			WithHelp(fmt.Sprintf("'%s' can only hold values up to %s", typeName, maxValue)).
			WithSuggestion(fmt.Sprintf("use '%s' instead to hold larger values", suggestedType)).
			WithNote("choose the smallest type that can hold your values to optimize gas usage").
			Build()
		a.addCompilerError(err)
	} else {
		err := errors.NewSemanticError(errors.ErrorNumericOverflow,
			fmt.Sprintf("value '%s' exceeds maximum for type '%s' (max: %s)", value, typeName, maxValue), pos).
			WithHelp(fmt.Sprintf("'%s' can only hold values up to %s", typeName, maxValue)).
			WithNote("this value exceeds the maximum for any supported numeric type").
			Build()
		a.addCompilerError(err)
	}
}

func (a *Analyzer) addDuplicateFieldError(fieldName, structName string, pos ast.Position) {
	err := errors.NewSemanticError(errors.ErrorDuplicateField,
		fmt.Sprintf("duplicate field '%s' in struct literal", fieldName), pos).
		WithHelp(fmt.Sprintf("field '%s' is specified multiple times", fieldName)).
		WithSuggestion("remove the duplicate field assignment").
		WithNote("each field in a struct literal can only be assigned once").
		Build()
	a.addCompilerError(err)
}

func (a *Analyzer) addMissingFieldError(fieldName, structName string, pos ast.Position) {
	err := errors.NewSemanticError(errors.ErrorMissingField,
		fmt.Sprintf("missing field '%s' in struct literal for '%s'", fieldName, structName), pos).
		WithHelp(fmt.Sprintf("struct '%s' requires field '%s' to be initialized", structName, fieldName)).
		WithSuggestion(fmt.Sprintf("add '%s: <value>' to the struct literal", fieldName)).
		WithNote("all struct fields must be initialized when creating a struct instance").
		Build()
	a.addCompilerError(err)
}

func (a *Analyzer) addModuleNotImportedError(moduleName string, pos ast.Position) {
	err := errors.NewSemanticError(errors.ErrorUndefinedModule,
		fmt.Sprintf("module '%s' is not imported", moduleName), pos).
		WithHelp(fmt.Sprintf("module '%s' must be imported before use", moduleName)).
		WithSuggestion(fmt.Sprintf("add 'use %s;' or 'use %s::{...}' at the top of the contract", moduleName, moduleName)).
		WithNote("standard library modules: std::evm, std::address, std::ascii, std::errors").
		Build()
	a.addCompilerError(err)
}

func (a *Analyzer) addVoidFunctionInExpressionError(funcName string, pos ast.Position) {
	err := errors.NewSemanticError(errors.ErrorVoidInExpression,
		fmt.Sprintf("function '%s' does not return a value but is used in a context that requires one", funcName), pos).
		WithHelp(fmt.Sprintf("'%s' is a void function and cannot be used in expressions", funcName)).
		WithSuggestion("call the function as a statement instead of using its return value").
		WithNote("void functions perform actions but don't return values").
		Build()
	a.addCompilerError(err)
}

func (a *Analyzer) addStorageAccessError(funcName, structName string, isWrite bool, pos ast.Position) {
	accessType := "reads"
	if isWrite {
		accessType = "writes"
	}

	err := errors.NewSemanticError(errors.ErrorStorageAccess,
		fmt.Sprintf("function '%s' accesses storage struct '%s' but does not declare it in %s clause", funcName, structName, accessType), pos).
		WithHelp(fmt.Sprintf("storage access must be explicitly declared in function signature")).
		WithSuggestion(fmt.Sprintf("add '%s(%s)' to the function signature", accessType, structName)).
		WithNote("explicit storage declarations help prevent unintended state modifications").
		Build()
	a.addCompilerError(err)
}
