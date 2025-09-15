package semantic

import (
	"testing"

	"kanso/internal/errors"
	"kanso/internal/parser"

	"github.com/stretchr/testify/assert"
)

// Helper function to get undefined function errors from source
func getUndefinedFunctionErrors(t *testing.T, source string) []errors.CompilerError {
	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	_ = analyzer.Analyze(contract)

	var undefinedFunctionErrors []errors.CompilerError
	for _, err := range analyzer.GetErrors() {
		if err.Code == "E0002" { // ErrorUndefinedFunction
			undefinedFunctionErrors = append(undefinedFunctionErrors, err)
		}
	}
	return undefinedFunctionErrors
}

// Helper function to check if suggestions contain a specific function name
func hasSuggestionFor(suggestions []errors.Suggestion, functionName string) bool {
	for _, suggestion := range suggestions {
		if containsSubstring(suggestion.Message, functionName) {
			return true
		}
	}
	return false
}

func TestImportedFunctionLookup(t *testing.T) {
	t.Run("SuggestsSimilarImportedFunctions", func(t *testing.T) {
		source := `contract TestSimilar {
			use std::evm::{sender, emit};
			
			ext fn test() {
				let result = sneder(); // Should suggest "sender"
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		errorMsg := undefinedFunctionErrors[0].Message
		assert.Contains(t, errorMsg, "sneder", "Error should mention the undefined function")
		assert.True(t, hasSuggestionFor(undefinedFunctionErrors[0].Suggestions, "sender"), "Error should suggest 'sender' as similar function")
	})

	t.Run("DoesNotSuggestVeryDifferentFunctions", func(t *testing.T) {
		source := `contract TestDifferent {
			use std::evm::{sender, emit};
			
			ext fn test() {
				let result = completely_different_name(); // Should not suggest sender/emit
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		assert.False(t, hasSuggestionFor(undefinedFunctionErrors[0].Suggestions, "sender"), "Should not suggest 'sender' for very different function name")
		assert.False(t, hasSuggestionFor(undefinedFunctionErrors[0].Suggestions, "emit"), "Should not suggest 'emit' for very different function name")
	})

	t.Run("SuggestsMultipleSimilarFunctions", func(t *testing.T) {
		source := `contract TestMultiple {
			use std::evm::{sender, emit};
			
			ext fn test() {
				let result = sende(); // Should suggest "sender"
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")
		assert.True(t, hasSuggestionFor(undefinedFunctionErrors[0].Suggestions, "sender"), "Should suggest 'sender' as similar function")
	})
}

func TestSimilarFunctionFinderDirectly(t *testing.T) {
	t.Run("FindsSimilarImportedFunctions", func(t *testing.T) {
		analyzer := NewAnalyzer()

		// Manually add imported functions for testing
		analyzer.context.functionRegistry.AddImportedFunction("sender", "std::evm")
		analyzer.context.functionRegistry.AddImportedFunction("emit", "std::evm")
		analyzer.context.functionRegistry.AddImportedFunction("balance", "std::address")

		// Test finding similar functions
		similar := analyzer.findSimilarFunctions("sneder")
		assert.Contains(t, similar, "sender", "Should find 'sender' as similar to 'sneder'")

		similar2 := analyzer.findSimilarFunctions("emitt")
		assert.Contains(t, similar2, "emit", "Should find 'emit' as similar to 'emitt'")

		similar3 := analyzer.findSimilarFunctions("balace")
		assert.Contains(t, similar3, "balance", "Should find 'balance' as similar to 'balace'")

		// Test that very different names don't get suggested
		similar4 := analyzer.findSimilarFunctions("completely_different")
		assert.Empty(t, similar4, "Should not find similar functions for very different names")
	})
}
