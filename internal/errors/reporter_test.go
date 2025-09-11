package errors

import (
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"kanso/internal/ast"
)

func TestErrorReporter(t *testing.T) {
	source := `contract Test {
    ext fn test() -> U256 {
        let x = unknownVar;
        return x;
    }
}`

	reporter := NewErrorReporter("test.ka", source)

	// Test basic error formatting
	err := UndefinedVariable("unknownVar", ast.Position{Line: 3, Column: 17}, []string{"knownVar", "anotherVar"})
	formatted := reporter.FormatError(err)

	// Should contain error level and code
	assert.Contains(t, formatted, "error["+ErrorUndefinedVariable+"]")
	assert.Contains(t, formatted, "undefined variable")
	assert.Contains(t, formatted, "unknownVar")

	// Should contain location
	assert.Contains(t, formatted, "test.ka:3:17")

	// Should contain suggestions
	assert.Contains(t, formatted, "did you mean")
	assert.Contains(t, formatted, "knownVar")
}

func TestUndefinedVariableError(t *testing.T) {
	pos := ast.Position{Line: 1, Column: 5}

	// Test with similar names
	err := UndefinedVariable("balace", pos, []string{"balance"})
	assert.Equal(t, ErrorUndefinedVariable, err.Code)
	assert.Contains(t, err.Message, "balace")
	assert.Len(t, err.Suggestions, 1)
	assert.Contains(t, err.Suggestions[0].Message, "did you mean 'balance'")

	// Test without similar names
	err = UndefinedVariable("xyz", pos, []string{})
	assert.Len(t, err.Suggestions, 1)
	assert.Contains(t, err.Suggestions[0].Message, "make sure the variable is declared")
}

func TestUndefinedFunctionError(t *testing.T) {
	pos := ast.Position{Line: 1, Column: 5}

	err := UndefinedFunction("sende", pos, []string{"sender"}, []string{"std::evm::{sender}"})
	assert.Equal(t, ErrorUndefinedFunction, err.Code)
	assert.Contains(t, err.Message, "sende")
	assert.Len(t, err.Suggestions, 2) // similar name + import suggestion
	assert.Contains(t, err.Suggestions[0].Message, "did you mean 'sender'")
	assert.Contains(t, err.Suggestions[1].Message, "try importing: use std::evm::{sender}")
}

func TestTypeMismatchError(t *testing.T) {
	pos := ast.Position{Line: 1, Column: 5}

	// Test numeric type mismatch with promotion
	err := TypeMismatch("U256", "U64", pos)
	assert.Equal(t, ErrorTypeMismatch, err.Code)
	assert.Contains(t, err.Message, "expected U256, found U64")
	assert.Len(t, err.Suggestions, 1)
	assert.Contains(t, err.Suggestions[0].Message, "compatible")

	// Test bool mismatch
	err = TypeMismatch("Bool", "U64", pos)
	assert.Contains(t, err.Suggestions[0].Message, "comparison operator")
}

func TestFieldNotFoundError(t *testing.T) {
	pos := ast.Position{Line: 1, Column: 5}

	err := FieldNotFound("Person", "nam", pos, []string{"name", "age", "email"})
	assert.Equal(t, ErrorFieldNotFound, err.Code)
	assert.Contains(t, err.Message, "struct 'Person' has no field 'nam'")
	assert.Len(t, err.Suggestions, 1)
	assert.Contains(t, err.Suggestions[0].Message, "did you mean 'name'")
	assert.Len(t, err.Notes, 1)
	assert.Contains(t, err.Notes[0], "available fields: name, age, email")
}

func TestWarningFormatting(t *testing.T) {
	source := `let unused = 42;`
	reporter := NewErrorReporter("test.ka", source)

	err := UnusedVariable("unused", ast.Position{Line: 1, Column: 5})
	formatted := reporter.FormatError(err)

	// Should be formatted as warning
	assert.Contains(t, formatted, "warning[W0001]")
	assert.Contains(t, formatted, "never used")
	assert.Contains(t, formatted, "prefix with underscore")
}

func TestErrorMarkerCreation(t *testing.T) {
	source := `let variable = value;`
	reporter := NewErrorReporter("test.ka", source)

	// Test marker creation
	marker := reporter.createMarker(5, 8, Error) // "variable" is 8 chars at column 5

	// Should have correct spacing and marker length
	spaces := strings.Count(marker, " ")
	assert.Equal(t, 4, spaces) // column 5 means 4 spaces before
	carets := strings.Count(marker, "^")
	assert.Equal(t, 8, carets) // 8 character length
}

func TestMultipleSuggestions(t *testing.T) {
	pos := ast.Position{Line: 1, Column: 5}

	err := UndefinedFunction("unknownFunc", pos,
		[]string{"knownFunc1", "knownFunc2"},
		[]string{"std::module::{func1}", "std::other::{func2}"})

	// Should have suggestions for similar names + import suggestions
	assert.True(t, len(err.Suggestions) >= 3)

	// Check that all suggestions are present
	suggestionTexts := make([]string, len(err.Suggestions))
	for i, s := range err.Suggestions {
		suggestionTexts[i] = s.Message
	}

	suggestionText := strings.Join(suggestionTexts, " ")
	assert.Contains(t, suggestionText, "knownFunc1")
	assert.Contains(t, suggestionText, "knownFunc2")
	assert.Contains(t, suggestionText, "std::module")
	assert.Contains(t, suggestionText, "std::other")
}

func TestLevenshteinDistance(t *testing.T) {
	// Test basic Levenshtein distance calculation
	assert.Equal(t, 0, levenshteinDistance("hello", "hello"))
	assert.Equal(t, 1, levenshteinDistance("hello", "hallo"))
	assert.Equal(t, 1, levenshteinDistance("hello", "helo")) // deletion is 1, not 2
	assert.Equal(t, 5, levenshteinDistance("hello", ""))
	assert.Equal(t, 3, levenshteinDistance("kitten", "sitting"))
}

func TestSimilarNameFinding(t *testing.T) {
	candidates := []string{"balance", "amount", "total", "balanceOf", "xyz"}

	// Should find similar names
	similar := findSimilarNames("balace", candidates)
	assert.Contains(t, similar, "balance")
	assert.NotContains(t, similar, "xyz") // too different

	// Should not find similar names if none are close enough
	similar = findSimilarNames("verydifferent", candidates)
	assert.Empty(t, similar)
}

func TestErrorLevels(t *testing.T) {
	source := `test`
	reporter := NewErrorReporter("test.ka", source)
	pos := ast.Position{Line: 1, Column: 1}

	// Test different error levels produce different colors
	errorErr := CompilerError{Level: Error, Message: "test error", Position: pos}
	warningErr := CompilerError{Level: Warning, Message: "test warning", Position: pos}

	errorFormatted := reporter.FormatError(errorErr)
	warningFormatted := reporter.FormatError(warningErr)

	assert.Contains(t, errorFormatted, "error:")
	assert.Contains(t, warningFormatted, "warning:")
}
