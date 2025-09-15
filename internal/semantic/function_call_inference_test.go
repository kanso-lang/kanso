package semantic

import (
	"testing"

	"kanso/internal/ast"

	"github.com/stretchr/testify/assert"
)

func TestFunctionCallInference(t *testing.T) {
	t.Run("EnhancedInferenceWithSignatureMatching", func(t *testing.T) {
		// Test that the enhanced inference works with signature matching
		source := `contract TestEnhanced {
			ext fn test() {
				// Should suggest both name-based and signature-based matches
				emitt(123); // Similar name + matching argument count
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		// Should have suggestions (could be emit, empty, etc.)
		suggestions := undefinedFunctionErrors[0].Suggestions
		assert.True(t, len(suggestions) > 0, "Should have suggestions based on enhanced inference")

		// Check that it includes signature-based suggestions
		hasImportSuggestions := false
		for _, suggestion := range suggestions {
			if containsSubstring(suggestion.Message, "try importing") {
				hasImportSuggestions = true
				break
			}
		}
		assert.True(t, hasImportSuggestions, "Should include import suggestions from enhanced inference")
	})
}

func TestFunctionsBySignature(t *testing.T) {
	t.Run("FindsLocalFunctionsBySignature", func(t *testing.T) {
		analyzer := NewAnalyzer()

		// Add a local function manually for testing
		analyzer.localFunctions["test_func"] = &ast.Function{
			Name: ast.Ident{Value: "test_func"},
			Params: []*ast.FunctionParam{
				{Name: ast.Ident{Value: "x"}, Type: &ast.VariableType{Name: ast.Ident{Value: "U256"}}},
			},
			Return: nil,
		}

		// Test finding functions with 1 argument
		matches := analyzer.findFunctionsBySignature("test_fn", 1, []string{"U256"})
		assert.Contains(t, matches, "test_func", "Should find local function with matching signature")
	})

	t.Run("FindsStandardLibraryFunctionsBySignature", func(t *testing.T) {
		analyzer := NewAnalyzer()

		// Test finding functions with 0 arguments
		matches := analyzer.findFunctionsBySignature("sndr", 0, []string{})

		// Should find sender function from std::evm
		found := false
		for _, match := range matches {
			if containsSubstring(match, "sender") {
				found = true
				break
			}
		}
		assert.True(t, found, "Should find sender function with matching signature")
	})

	t.Run("DoesNotFindFunctionsWithWrongSignature", func(t *testing.T) {
		analyzer := NewAnalyzer()

		// Test finding functions with wrong argument count
		matches := analyzer.findFunctionsBySignature("sender", 5, []string{"U256", "U256", "U256", "U256", "U256"})

		// Should not find sender function (it has 0 args, not 5)
		found := false
		for _, match := range matches {
			if containsSubstring(match, "sender") {
				found = true
				break
			}
		}
		assert.False(t, found, "Should not find sender function with wrong argument count")
	})
}
