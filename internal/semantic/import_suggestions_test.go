package semantic

import (
	"testing"

	"kanso/internal/errors"

	"github.com/stretchr/testify/assert"
)

// Helper function to check if import suggestions contain a specific suggestion
func hasImportSuggestion(suggestions []errors.Suggestion, suggestion string) bool {
	for _, s := range suggestions {
		if containsSubstring(s.Message, suggestion) {
			return true
		}
	}
	return false
}

func TestImportSuggestions(t *testing.T) {
	t.Run("SuggestsImportForSimilarStandardLibraryFunction", func(t *testing.T) {
		source := `contract TestImport {
			ext fn test() {
				let result = sneder(); // Should suggest importing sender from std::evm
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		errorMsg := undefinedFunctionErrors[0].Message
		assert.Contains(t, errorMsg, "sneder", "Error should mention the undefined function")

		// Check that it suggests importing sender from std::evm
		assert.True(t, hasImportSuggestion(undefinedFunctionErrors[0].Suggestions, "std::evm::{sender}"),
			"Should suggest importing sender from std::evm")
	})

	t.Run("SuggestsImportForExactMatchInStandardLibrary", func(t *testing.T) {
		source := `contract TestImport {
			ext fn test() {
				let result = sender(); // Should suggest importing sender
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		// Check that it suggests importing sender from std::evm
		assert.True(t, hasImportSuggestion(undefinedFunctionErrors[0].Suggestions, "std::evm::{sender}"),
			"Should suggest importing sender from std::evm")
	})

	t.Run("SuggestsImportForEmit", func(t *testing.T) {
		source := `contract TestImport {
			ext fn test() {
				emitt(); // Should suggest importing emit
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		// Check that it suggests importing emit from std::evm
		assert.True(t, hasImportSuggestion(undefinedFunctionErrors[0].Suggestions, "std::evm::{emit}"),
			"Should suggest importing emit from std::evm")
	})

	t.Run("SuggestsImportForAddressFunctions", func(t *testing.T) {
		source := `contract TestImport {
			ext fn test() {
				let addr = zro(); // Should suggest zero from std::address
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		// Check that it suggests importing zero from std::address
		assert.True(t, hasImportSuggestion(undefinedFunctionErrors[0].Suggestions, "std::address::{zero}"),
			"Should suggest importing zero from std::address")
	})

	t.Run("DoesNotSuggestImportForVeryDifferentFunction", func(t *testing.T) {
		source := `contract TestImport {
			ext fn test() {
				completely_different_function(); // Should not suggest any standard library imports
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		// Check that no standard library imports are suggested
		hasStdlibSuggestions := false
		for _, suggestion := range undefinedFunctionErrors[0].Suggestions {
			if containsSubstring(suggestion.Message, "use std::") {
				hasStdlibSuggestions = true
				break
			}
		}
		assert.False(t, hasStdlibSuggestions, "Should not suggest standard library imports for very different function names")
	})

	t.Run("SuggestsMultipleImportsForSimilarFunctions", func(t *testing.T) {
		source := `contract TestImport {
			ext fn test() {
				let result = send(); // Similar to both sender and other functions
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		// Should have at least one suggestion (might have more depending on standard library)
		assert.True(t, len(undefinedFunctionErrors[0].Suggestions) > 0, "Should have at least one suggestion")
	})
}

func TestStandardLibraryFunctionFinder(t *testing.T) {
	t.Run("FindsStandardLibraryFunctions", func(t *testing.T) {
		analyzer := NewAnalyzer()

		// Test finding similar functions in standard library
		imports1 := analyzer.findPossibleImports("sneder")
		assert.Contains(t, imports1, "std::evm::{sender}", "Should suggest importing sender from std::evm")

		imports2 := analyzer.findPossibleImports("emitt")
		assert.Contains(t, imports2, "std::evm::{emit}", "Should suggest importing emit from std::evm")

		imports3 := analyzer.findPossibleImports("zro")
		assert.Contains(t, imports3, "std::address::{zero}", "Should suggest importing zero from std::address")

		// Test that very different names don't get suggested
		imports4 := analyzer.findPossibleImports("completely_different")
		assert.Empty(t, imports4, "Should not find similar functions for very different names")
	})
}
