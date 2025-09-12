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
