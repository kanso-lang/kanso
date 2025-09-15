package semantic

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestSmartImportSuggestions(t *testing.T) {
	t.Run("SuggestsExtendingExistingImport", func(t *testing.T) {
		source := `contract TestSmartImport {
			use std::evm::{emit};
			
			ext fn test() {
				snder(); // Should suggest extending existing import
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		// Check that it suggests extending the existing import
		foundExtendedImport := false
		for _, suggestion := range undefinedFunctionErrors[0].Suggestions {
			if containsSubstring(suggestion.Message, "std::evm::{emit, sender}") {
				foundExtendedImport = true
				break
			}
		}
		assert.True(t, foundExtendedImport, "Should suggest extending existing std::evm import with sender")
	})

	t.Run("SuggestsAlphabeticalOrder", func(t *testing.T) {
		source := `contract TestAlphabetical {
			use std::evm::{sender};
			
			ext fn test() {
				emitt(123); // Should suggest alphabetical order: emit, sender
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		// Check that it suggests alphabetical order (emit before sender)
		foundAlphabetical := false
		for _, suggestion := range undefinedFunctionErrors[0].Suggestions {
			if containsSubstring(suggestion.Message, "std::evm::{emit, sender}") {
				foundAlphabetical = true
				break
			}
		}
		assert.True(t, foundAlphabetical, "Should suggest alphabetical order: emit, sender")
	})

	t.Run("SuggestsNewImportWhenNoExisting", func(t *testing.T) {
		source := `contract TestNewImport {
			use std::evm::{emit};
			
			ext fn test() {
				zro(); // Should suggest new import since no std::address import exists
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		// Check that it suggests a new import for std::address
		foundNewImport := false
		for _, suggestion := range undefinedFunctionErrors[0].Suggestions {
			if containsSubstring(suggestion.Message, "std::address::{zero}") {
				foundNewImport = true
				break
			}
		}
		assert.True(t, foundNewImport, "Should suggest new import for std::address::zero")
	})
}
