package semantic

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestFunctionCallInferenceImprovement(t *testing.T) {
	t.Run("InfersLocalFunctionReturnTypeInComplexExpression", func(t *testing.T) {
		// This test verifies that function call inference works in fallback scenarios
		// where the main type inference might fail and recovery is needed
		source := `contract TestInference {
			fn get_balance() -> U256 {
				100
			}
			
			fn test() -> Bool {
				// Complex expression that might trigger type inference recovery
				let result = get_balance() + 50;
				result > 0
			}
		}`

		errors := getAllSemanticErrors(t, source)

		// Should not have any type-related errors
		for _, err := range errors {
			assert.False(t, containsSubstring(err.Message, "type mismatch"),
				"Should not have type mismatch errors with improved function call inference")
			assert.False(t, containsSubstring(err.Message, "cannot infer type"),
				"Should not have type inference errors")
		}
	})

	t.Run("InfersImportedFunctionReturnTypeInRecovery", func(t *testing.T) {
		// Test that imported function calls are properly inferred in fallback scenarios
		source := `contract TestImportedInference {
			use std::evm::{sender};
			
			fn test() -> Bool {
				// Expression that might need type inference recovery
				sender() != address::zero()
			}
		}`

		errors := getAllSemanticErrors(t, source)

		// Should not have type inference errors for sender() call
		for _, err := range errors {
			assert.False(t, containsSubstring(err.Message, "type mismatch"),
				"Should infer sender() returns Address type correctly")
		}
	})

	t.Run("HandlesVoidFunctionCallsInRecovery", func(t *testing.T) {
		// Test that void function calls are handled correctly in recovery scenarios
		source := `contract TestVoidInference {
			use std::evm::{emit};
			
			fn helper() {
				// void function
			}
			
			fn test() {
				helper(); // Should not cause type inference issues
				emit(Transfer{from: address::zero(), to: address::zero(), value: 100});
			}
		}`

		errors := getAllSemanticErrors(t, source)

		// Filter out unrelated errors, focus on type inference
		hasTypeInferenceError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "cannot infer") ||
				containsSubstring(err.Message, "unknown type") {
				hasTypeInferenceError = true
			}
		}

		assert.False(t, hasTypeInferenceError,
			"Should handle void function calls without type inference errors")
	})
}
