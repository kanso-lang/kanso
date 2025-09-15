package semantic

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestRedundantSuggestionElimination(t *testing.T) {
	t.Run("EliminatesRedundantStandaloneImport", func(t *testing.T) {
		source := `contract TestRedundancy {
			use std::evm::{emit};
			
			ext fn test() {
				sender(); // Should only suggest extending existing import, not both options
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		// Should have only one import suggestion (the extended one)
		importSuggestions := make([]string, 0)
		for _, suggestion := range undefinedFunctionErrors[0].Suggestions {
			if containsSubstring(suggestion.Message, "try importing") {
				importSuggestions = append(importSuggestions, suggestion.Message)
			}
		}

		assert.Len(t, importSuggestions, 1, "Should have exactly one import suggestion")

		// The single suggestion should be the extended import
		extendedSuggestion := importSuggestions[0]
		assert.True(t, containsSubstring(extendedSuggestion, "std::evm::{emit, sender}"),
			"Should suggest extended import with alphabetical order")

		// Should NOT suggest standalone import
		assert.False(t, containsSubstring(extendedSuggestion, "std::evm::{sender}"),
			"Should not suggest redundant standalone import")
	})

	t.Run("StillSuggestsNewImportWhenNoExistingModuleImport", func(t *testing.T) {
		source := `contract TestNewModule {
			use std::evm::{emit};
			
			ext fn test() {
				zro(); // Should suggest new import since std::address not imported
			}
		}`

		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have one undefined function error")

		// Should suggest new import for different module
		foundNewImport := false
		for _, suggestion := range undefinedFunctionErrors[0].Suggestions {
			if containsSubstring(suggestion.Message, "std::address::{zero}") {
				foundNewImport = true
				break
			}
		}
		assert.True(t, foundNewImport, "Should suggest new import for different module")
	})
}
