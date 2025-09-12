package errors

import (
	"fmt"
	"strings"

	"kanso/internal/ast"
)

// SemanticErrorBuilder provides a fluent interface for creating semantic errors with suggestions
type SemanticErrorBuilder struct {
	err CompilerError
}

// NewSemanticError creates a new semantic error builder
func NewSemanticError(code, message string, pos ast.Position) *SemanticErrorBuilder {
	return &SemanticErrorBuilder{
		err: CompilerError{
			Level:    Error,
			Code:     code,
			Message:  message,
			Position: pos,
			Length:   1,
		},
	}
}

// NewSemanticWarning creates a new semantic warning builder
func NewSemanticWarning(code, message string, pos ast.Position) *SemanticErrorBuilder {
	return &SemanticErrorBuilder{
		err: CompilerError{
			Level:    Warning,
			Code:     code,
			Message:  message,
			Position: pos,
			Length:   1,
		},
	}
}

// WithLength sets the length of the error span
func (b *SemanticErrorBuilder) WithLength(length int) *SemanticErrorBuilder {
	b.err.Length = length
	return b
}

// WithSuggestion adds a suggestion to the error
func (b *SemanticErrorBuilder) WithSuggestion(message string) *SemanticErrorBuilder {
	b.err.Suggestions = append(b.err.Suggestions, Suggestion{Message: message})
	return b
}

// WithReplacement adds a suggestion with replacement text
func (b *SemanticErrorBuilder) WithReplacement(message, replacement string, pos ast.Position, length int) *SemanticErrorBuilder {
	b.err.Suggestions = append(b.err.Suggestions, Suggestion{
		Message:     message,
		Replacement: replacement,
		Position:    pos,
		Length:      length,
	})
	return b
}

// WithNote adds a note to the error
func (b *SemanticErrorBuilder) WithNote(note string) *SemanticErrorBuilder {
	b.err.Notes = append(b.err.Notes, note)
	return b
}

// WithHelp adds help text to the error
func (b *SemanticErrorBuilder) WithHelp(help string) *SemanticErrorBuilder {
	b.err.HelpText = help
	return b
}

// Build returns the completed compiler error
func (b *SemanticErrorBuilder) Build() CompilerError {
	return b.err
}

// Common semantic error constructors with suggestions

// UndefinedVariable creates an error for undefined variables with suggestions
func UndefinedVariable(name string, pos ast.Position, similarNames []string) CompilerError {
	builder := NewSemanticError(ErrorUndefinedVariable, fmt.Sprintf("undefined variable '%s'", name), pos).
		WithLength(len(name))

	if len(similarNames) > 0 {
		if len(similarNames) == 1 {
			builder = builder.WithSuggestion(fmt.Sprintf("did you mean '%s'?", similarNames[0]))
		} else {
			suggestions := strings.Join(similarNames, "', '")
			builder = builder.WithSuggestion(fmt.Sprintf("did you mean one of: '%s'?", suggestions))
		}
	} else {
		builder = builder.WithSuggestion("make sure the variable is declared before use").
			WithNote("variables must be declared with 'let' or 'let mut'")
	}

	return builder.Build()
}

// UndefinedFunction creates an error for undefined functions with suggestions
func UndefinedFunction(name string, pos ast.Position, similarNames []string, availableImports []string) CompilerError {
	builder := NewSemanticError(ErrorUndefinedFunction, fmt.Sprintf("function '%s' is not imported or defined", name), pos).
		WithLength(len(name))

	if len(similarNames) > 0 {
		if len(similarNames) == 1 {
			builder = builder.WithSuggestion(fmt.Sprintf("did you mean '%s'?", similarNames[0]))
		} else {
			suggestions := strings.Join(similarNames, "', '")
			builder = builder.WithSuggestion(fmt.Sprintf("did you mean one of: '%s'?", suggestions))
		}
	}

	if len(availableImports) > 0 {
		for _, imp := range availableImports {
			builder = builder.WithSuggestion(fmt.Sprintf("try importing: use %s;", imp))
		}
	}

	return builder.WithHelp("functions must be either defined locally or imported from standard library modules").Build()
}

// TypeMismatch creates an error for type mismatches with conversion suggestions
func TypeMismatch(expected, actual string, pos ast.Position) CompilerError {
	builder := NewSemanticError(ErrorTypeMismatch, fmt.Sprintf("type mismatch: expected %s, found %s", expected, actual), pos)

	// Suggest type conversions for common cases
	if isNumericType(expected) && isNumericType(actual) {
		if canPromoteType(actual, expected) {
			builder = builder.WithSuggestion("the types are compatible, this should work automatically")
		} else {
			builder = builder.WithSuggestion(fmt.Sprintf("consider explicit conversion or use a %s literal", expected)).
				WithNote("narrowing conversions require explicit casts to prevent data loss")
		}
	} else if expected == "Bool" && actual != "Bool" {
		builder = builder.WithSuggestion("use a comparison operator to create a boolean value").
			WithReplacement("try using a comparison", fmt.Sprintf("(%s comparison)", actual), pos, 0)
	}

	return builder.Build()
}

// UnusedVariable creates a warning for unused variables
func UnusedVariable(name string, pos ast.Position) CompilerError {
	return NewSemanticWarning(WarningUnusedVariable, fmt.Sprintf("variable '%s' is declared but never used", name), pos).
		WithLength(len(name)).
		WithSuggestion(fmt.Sprintf("prefix with underscore to silence: '_%s'", name)).
		WithSuggestion("remove the variable declaration if it's not needed").
		WithHelp("unused variables can indicate dead code or logic errors").
		Build()
}

// UnreachableCode creates a warning for unreachable code
func UnreachableCode(pos ast.Position) CompilerError {
	return NewSemanticWarning(WarningUnreachableCode, "unreachable code", pos).
		WithSuggestion("remove the unreachable code").
		WithSuggestion("move the return statement to after this code if it should be executed").
		WithNote("code after a return statement will never be executed").
		Build()
}

// MissingReturnStatement creates an error for missing return statements
func MissingReturnStatement(functionName, returnType string, pos ast.Position) CompilerError {
	builder := NewSemanticError(ErrorInvalidReturnType, fmt.Sprintf("function '%s' must return a value of type %s", functionName, returnType), pos)

	if returnType != "" {
		builder = builder.WithSuggestion(fmt.Sprintf("add 'return <value>;' where <value> is of type %s", returnType))
	}

	return builder.WithSuggestion("use an expression without semicolon as the last statement (tail expression)").
		WithNote("functions with return types must return a value on all code paths").
		Build()
}

// FieldNotFound creates an error for missing struct fields with suggestions
func FieldNotFound(structName, fieldName string, pos ast.Position, availableFields []string) CompilerError {
	builder := NewSemanticError(ErrorFieldNotFound, fmt.Sprintf("struct '%s' has no field '%s'", structName, fieldName), pos).
		WithLength(len(fieldName))

	if len(availableFields) > 0 {
		// Find similar field names
		similar := findSimilarNames(fieldName, availableFields)
		if len(similar) > 0 {
			if len(similar) == 1 {
				builder = builder.WithSuggestion(fmt.Sprintf("did you mean '%s'?", similar[0]))
			} else {
				suggestions := strings.Join(similar, "', '")
				builder = builder.WithSuggestion(fmt.Sprintf("did you mean one of: '%s'?", suggestions))
			}
		}

		// Show available fields
		fields := strings.Join(availableFields, ", ")
		builder = builder.WithNote(fmt.Sprintf("available fields: %s", fields))
	}

	return builder.Build()
}

// DuplicateField creates an error for duplicate struct literal fields
func DuplicateField(fieldName string, pos ast.Position) CompilerError {
	return NewSemanticError(ErrorDuplicateField, fmt.Sprintf("duplicate field '%s' in struct literal", fieldName), pos).
		WithLength(len(fieldName)).
		WithSuggestion("remove one of the duplicate field assignments").
		WithNote("each field can only be specified once in a struct literal").
		Build()
}

// MissingField creates an error for missing required struct fields
func MissingField(structName, fieldName string, pos ast.Position) CompilerError {
	return NewSemanticError(ErrorMissingField, fmt.Sprintf("missing field '%s' in struct literal for '%s'", fieldName, structName), pos).
		WithSuggestion(fmt.Sprintf("add the missing field: %s: <value>", fieldName)).
		WithNote("all struct fields must be specified in struct literals").
		Build()
}

// InvalidOperation creates an error for invalid operations with type-specific suggestions
func InvalidOperation(op, leftType, rightType string, pos ast.Position) CompilerError {
	builder := NewSemanticError(ErrorInvalidBinaryOperation, fmt.Sprintf("invalid operation: %s %s %s", leftType, op, rightType), pos)

	// Provide operation-specific suggestions
	switch op {
	case "+", "-", "*", "/", "%":
		if !isNumericType(leftType) || !isNumericType(rightType) {
			builder = builder.WithSuggestion("arithmetic operations require numeric types").
				WithNote("numeric types are: U8, U16, U32, U64, U128, U256")
		}
	case "&&", "||":
		builder = builder.WithSuggestion("logical operations require boolean operands").
			WithSuggestion("use comparison operators (==, !=, <, >, <=, >=) to create boolean values")
	case "==", "!=", "<", "<=", ">", ">=":
		builder = builder.WithSuggestion("comparison operands must be of compatible types")
	}

	return builder.Build()
}

// Helper functions

func isNumericType(typeName string) bool {
	numericTypes := map[string]bool{
		"U8": true, "U16": true, "U32": true, "U64": true, "U128": true, "U256": true,
	}
	return numericTypes[typeName]
}

func canPromoteType(from, to string) bool {
	typeOrder := map[string]int{
		"U8": 1, "U16": 2, "U32": 3, "U64": 4, "U128": 5, "U256": 6,
	}

	fromOrder, fromExists := typeOrder[from]
	toOrder, toExists := typeOrder[to]

	return fromExists && toExists && fromOrder <= toOrder
}

func findSimilarNames(target string, candidates []string) []string {
	var similar []string

	for _, candidate := range candidates {
		if levenshteinDistance(target, candidate) <= 2 && len(candidate) > 2 {
			similar = append(similar, candidate)
		}
	}

	return similar
}

// Simple Levenshtein distance implementation for finding similar names
func levenshteinDistance(a, b string) int {
	if len(a) == 0 {
		return len(b)
	}
	if len(b) == 0 {
		return len(a)
	}

	// Create matrix
	matrix := make([][]int, len(a)+1)
	for i := range matrix {
		matrix[i] = make([]int, len(b)+1)
	}

	// Initialize first row and column
	for i := 0; i <= len(a); i++ {
		matrix[i][0] = i
	}
	for j := 0; j <= len(b); j++ {
		matrix[0][j] = j
	}

	// Fill the matrix
	for i := 1; i <= len(a); i++ {
		for j := 1; j <= len(b); j++ {
			cost := 0
			if a[i-1] != b[j-1] {
				cost = 1
			}

			matrix[i][j] = min3(
				matrix[i-1][j]+1,      // deletion
				matrix[i][j-1]+1,      // insertion
				matrix[i-1][j-1]+cost, // substitution
			)
		}
	}

	return matrix[len(a)][len(b)]
}

// Flow control error functions

// MissingReturn creates an error for functions that declare a return type but have no return statement
func MissingReturn(functionName, returnType string, pos ast.Position) CompilerError {
	message := fmt.Sprintf("function '%s' declares return type '%s' but has no return statement", functionName, returnType)
	return NewSemanticError(ErrorMissingReturn, message, pos).
		WithSuggestion(fmt.Sprintf("add a return statement that returns a value of type '%s'", returnType)).
		WithSuggestion("or add a tail expression (expression without semicolon at the end)").
		WithHelp("functions with return types must return a value on all execution paths").
		Build()
}

// NewUnreachableCode creates an error for code that cannot be reached (updated version)
func NewUnreachableCode(pos ast.Position) CompilerError {
	return NewSemanticWarning(ErrorUnreachableCode, "unreachable code", pos).
		WithSuggestion("remove this code").
		WithSuggestion("or move it before the return statement").
		Build()
}

// DuplicateDeclaration creates an error for duplicate declarations
func DuplicateDeclaration(name string, pos ast.Position) CompilerError {
	return NewSemanticError(ErrorDuplicateDeclaration, fmt.Sprintf("duplicate declaration: %s", name), pos).
		WithSuggestion(fmt.Sprintf("rename the duplicate '%s' to a unique name", name)).
		WithSuggestion("or remove the duplicate declaration").
		WithNote("identifiers must be unique within their scope").
		Build()
}

// InvalidAttribute creates an error for invalid attributes
func InvalidAttribute(attributeName string, pos ast.Position) CompilerError {
	validAttributes := []string{"storage", "event", "create"}

	builder := NewSemanticError(ErrorInvalidAttribute, fmt.Sprintf("invalid attribute: %s", attributeName), pos).
		WithHelp("attributes must be one of: #[storage], #[event], or #[create]")

	// Find similar attributes for suggestions
	for _, valid := range validAttributes {
		if levenshteinDistance(attributeName, valid) <= 2 {
			builder = builder.WithSuggestion(fmt.Sprintf("did you mean '#[%s]'?", valid))
		}
	}

	return builder.Build()
}

// InvalidReadsWrites creates an error for invalid reads/writes clauses
func InvalidReadsWrites(message string, pos ast.Position) CompilerError {
	return NewSemanticError(ErrorInvalidReadsWrites, message, pos).
		WithHelp("reads/writes clauses must reference storage structs").
		WithSuggestion("ensure the referenced struct has #[storage] attribute").
		Build()
}

// InvalidConstructor creates an error for invalid constructor definitions
func InvalidConstructor(message string, pos ast.Position) CompilerError {
	return NewSemanticError(ErrorInvalidConstructor, message, pos).
		WithHelp("constructor functions must have #[create] attribute").
		WithSuggestion("add a writes clause to specify storage access").
		WithSuggestion("remove return type - constructors don't return values").
		Build()
}

// InvalidArguments creates an error for function call argument mismatches
func InvalidArguments(functionName string, expected, actual int, pos ast.Position) CompilerError {
	return NewSemanticError(ErrorInvalidArguments,
		fmt.Sprintf("function '%s' expects %d arguments, got %d", functionName, expected, actual), pos).
		WithSuggestion(fmt.Sprintf("provide exactly %d argument(s)", expected)).
		WithHelp("check the function signature for the correct number of parameters").
		Build()
}

// InvalidAssignment creates an error for invalid assignment operations
func InvalidAssignment(message string, pos ast.Position) CompilerError {
	return NewSemanticError(ErrorInvalidAssignment, message, pos).
		WithHelp("assignments must be to assignable expressions").
		WithSuggestion("ensure the target is a variable, field access, or index expression").
		WithSuggestion("check that the variable is declared as 'let mut' for mutability").
		Build()
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
