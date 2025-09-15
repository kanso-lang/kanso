package semantic

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

// TestTupleTypeSystem comprehensively tests tuple type representation, inference, and promotion
func TestTupleTypeSystem(t *testing.T) {

	// ===== TYPE REPRESENTATION TESTS =====

	t.Run("InfersTupleElementTypes", func(t *testing.T) {
		source := `contract TestTupleInference {
    fn test() -> (U256, Bool) {
        (256, true)
    }
}`

		errors := getAllSemanticErrors(t, source)

		// Should not have type mismatch errors with proper tuple inference
		for _, err := range errors {
			assert.False(t, containsSubstring(err.Message, "type mismatch"),
				"Should not have type mismatch with proper tuple type inference")
		}
	})

	t.Run("DetectsTupleElementTypeMismatch", func(t *testing.T) {
		source := `contract TestTupleMismatch {
    fn test() -> (U256, Bool) {
        (true, 42)
    }
}`

		errors := getAllSemanticErrors(t, source)

		// Should detect the type mismatch with proper tuple type representation
		foundMismatch := false
		for _, err := range errors {
			if containsSubstring(err.Message, "type mismatch") ||
				containsSubstring(err.Message, "expects return type") ||
				containsSubstring(err.Message, "expected") {
				foundMismatch = true
			}
		}

		assert.True(t, foundMismatch, "Should detect tuple element type mismatch")
	})

	t.Run("HandlesMixedTupleTypes", func(t *testing.T) {
		source := `contract TestMixedTuple {
			use std::evm::{sender};
			
			fn test() -> (Address, U256, Bool) {
				(sender(), 100, false)
			}
		}`

		errors := getAllSemanticErrors(t, source)

		// Should handle complex tuple with different element types
		hasTypeError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "type mismatch") ||
				containsSubstring(err.Message, "cannot infer") {
				hasTypeError = true
			}
		}

		assert.False(t, hasTypeError,
			"Should handle mixed tuple types (Address, U256, Bool) correctly")
	})

	t.Run("HandlesNestedTuples", func(t *testing.T) {
		source := `contract TestNestedTuples {
			fn inner() -> (U256, Bool) {
				(42, true)
			}
			
			fn test() {
				let nested = (inner(), 100);  // ((U256, Bool), U256)
			}
		}`

		errors := getAllSemanticErrors(t, source)

		// Should handle nested tuple structures without major errors
		hasTypeInferenceError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "cannot infer") {
				hasTypeInferenceError = true
			}
		}

		assert.False(t, hasTypeInferenceError,
			"Should handle nested tuple structures")
	})

	t.Run("HandlesEmptyTuple", func(t *testing.T) {
		source := `contract TestEmptyTuple {
			fn test() {
				let empty = ();
			}
		}`

		errors := getAllSemanticErrors(t, source)

		// Should handle empty tuples without crashing
		hasCrashError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "panic") ||
				containsSubstring(err.Message, "crash") {
				hasCrashError = true
			}
		}

		assert.False(t, hasCrashError, "Should handle empty tuples gracefully")
	})

	// ===== NUMERIC PROMOTION TESTS =====

	t.Run("PromotesNumericLiteralsInTuples", func(t *testing.T) {
		source := `contract TestPromotion {
    ext fn test() -> (U256, Bool) {
        (42, true)
    }
}`

		errors := getAllSemanticErrors(t, source)

		hasPromotionError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "expects return type") {
				hasPromotionError = true
			}
		}

		assert.False(t, hasPromotionError,
			"Should promote U8 literal (42) to U256 in tuple")
	})

	t.Run("RejectsInvalidTypeConversions", func(t *testing.T) {
		source := `contract TestInvalidConversion {
    ext fn test() -> (U256, Bool) {
        (true, 42)
    }
}`

		errors := getAllSemanticErrors(t, source)

		hasTypeError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "expects return type") {
				hasTypeError = true
			}
		}

		assert.True(t, hasTypeError,
			"Should reject invalid Bool -> U256 conversion in tuple")
	})

	t.Run("ValidatesPromotionHierarchy", func(t *testing.T) {
		// Test various promotion scenarios
		testCases := []struct {
			name          string
			returnType    string
			tupleValue    string
			shouldPromote bool
		}{
			{"U8ToU16", "(U16, Bool)", "(255, true)", true},
			{"U8ToU256", "(U256, Bool)", "(42, true)", true},
			{"U16ToU32", "(U32, Bool)", "(65535, true)", true},
			{"U64ToU256", "(U256, Bool)", "(18446744073709551615, true)", true},
			{"AddressToU256", "(U256, Bool)", "(address::zero(), true)", false}, // Should fail
		}

		for _, tc := range testCases {
			t.Run(tc.name, func(t *testing.T) {
				source := `contract Test {
    use std::address;
    ext fn test() -> ` + tc.returnType + ` {
        ` + tc.tupleValue + `
    }
}`

				errors := getAllSemanticErrors(t, source)

				hasTypeError := false
				for _, err := range errors {
					if containsSubstring(err.Message, "expects return type") ||
						containsSubstring(err.Message, "type mismatch") {
						hasTypeError = true
					}
				}

				if tc.shouldPromote {
					assert.False(t, hasTypeError,
						"Should allow promotion for %s", tc.name)
				} else {
					assert.True(t, hasTypeError,
						"Should reject invalid conversion for %s", tc.name)
				}
			})
		}
	})

	// ===== TYPE DISPLAY TESTS =====

	t.Run("DisplaysTupleTypesWithProperSyntax", func(t *testing.T) {
		// This test verifies that tuple types are displayed as (Type1, Type2)
		// rather than Tuple<Type1, Type2>
		source := `contract TestDisplay {
    fn test() -> (U256, Bool) {
        (true, 42)
    }
}`

		errors := getAllSemanticErrors(t, source)

		foundTupleDisplay := false
		for _, err := range errors {
			// Look for the proper (Type1, Type2) syntax in error messages
			if containsSubstring(err.Message, "(Bool, U8)") &&
				containsSubstring(err.Message, "(U256, Bool)") {
				foundTupleDisplay = true
			}
		}

		assert.True(t, foundTupleDisplay,
			"Should display tuple types using (Type1, Type2) syntax in error messages")
	})
}
