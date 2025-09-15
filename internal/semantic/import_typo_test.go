package semantic

import (
	"testing"

	"kanso/internal/errors"
	"kanso/internal/parser"

	"github.com/stretchr/testify/assert"
)

// Helper function to get all semantic errors from source
func getAllSemanticErrors(t *testing.T, source string) []errors.CompilerError {
	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	_ = analyzer.Analyze(contract)

	return analyzer.GetErrors()
}

func TestImportTypoDetection(t *testing.T) {
	t.Run("DetectsTypoInImportStatement", func(t *testing.T) {
		source := `contract TestTypo {
			use std::evm::{sendr, emit}; // sendr is a typo
			
			ext fn test() {
				sender(); // Should suggest correct import without the typo
			}
		}`

		errors := getAllSemanticErrors(t, source)

		// Should have import error for the typo
		foundImportError := false
		foundSuggestion := false
		for _, err := range errors {
			if containsSubstring(err.Message, "unknown function or type 'sendr'") {
				foundImportError = true
			}
			if containsSubstring(err.Message, "did you mean: sender") {
				foundSuggestion = true
			}
		}

		assert.True(t, foundImportError, "Should detect typo 'sendr' as unknown function")
		assert.True(t, foundSuggestion, "Should suggest correct spelling 'sender'")

		// Check undefined function errors (sender not imported due to typo)
		undefinedFunctionErrors := getUndefinedFunctionErrors(t, source)
		assert.Len(t, undefinedFunctionErrors, 1, "Should have undefined function error for sender")

		// Should suggest correct import without including the typo
		suggestions := undefinedFunctionErrors[0].Suggestions
		foundCorrectSuggestion := false
		foundTypoInSuggestion := false

		for _, suggestion := range suggestions {
			if containsSubstring(suggestion.Message, "use std::evm::{emit, sender}") {
				foundCorrectSuggestion = true
			}
			if containsSubstring(suggestion.Message, "sendr") {
				foundTypoInSuggestion = true
			}
		}

		assert.True(t, foundCorrectSuggestion, "Should suggest correct extended import")
		assert.False(t, foundTypoInSuggestion, "Should not include typo in import suggestion")
	})

	t.Run("DetectsMultipleTyposInImport", func(t *testing.T) {
		source := `contract TestMultipleTypos {
			use std::evm::{snder, emt}; // Multiple typos
			
			ext fn test() {
				sender();
				emit(Transfer{from: address::zero(), to: address::zero(), amount: 100});
			}
		}`

		errors := getAllSemanticErrors(t, source)

		// Should detect both typos
		foundSnderError := false
		foundEmtError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "unknown function or type 'snder'") {
				foundSnderError = true
			}
			if containsSubstring(err.Message, "unknown function or type 'emt'") {
				foundEmtError = true
			}
		}

		assert.True(t, foundSnderError, "Should detect 'snder' typo")
		assert.True(t, foundEmtError, "Should detect 'emt' typo")
	})

	t.Run("ValidImportNoErrors", func(t *testing.T) {
		source := `contract TestValid {
			use std::evm::{sender, emit};
			
			ext fn test() {
				sender();
				emit(Transfer{from: address::zero(), to: address::zero(), amount: 100});
			}
		}`

		errors := getAllSemanticErrors(t, source)

		// Should not have any import-related errors
		for _, err := range errors {
			assert.False(t, containsSubstring(err.Message, "unknown function or type"),
				"Should not have unknown function/type errors for valid imports")
		}
	})
}
