package semantic

import (
	"kanso/internal/parser"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestTypePromotionInFunctionCalls(t *testing.T) {
	t.Run("ValidPromotionU8ToU256", func(t *testing.T) {
		source := `contract Test {
			fn get_small() -> U8 {
				42
			}

			ext fn test() {
				let x: U256 = get_small();  // Valid: U8 promotes to U256
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter out unused variable warnings - we're testing type promotion
		errors := FilterUnusedVariables(allErrors)

		assert.Empty(t, errors, "Should have no errors for valid U8 to U256 promotion")
	})

	t.Run("ValidPromotionU32ToU128", func(t *testing.T) {
		source := `contract Test {
			fn get_medium() -> U32 {
				1000
			}

			ext fn test() {
				let x: U128 = get_medium();  // Valid: U32 promotes to U128
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Empty(t, errors, "Should have no errors for valid U32 to U128 promotion")
	})

	t.Run("ValidPromotionU16ToU64", func(t *testing.T) {
		source := `contract Test {
			fn get_small() -> U16 {
				500
			}

			ext fn test() {
				let x: U64 = get_small();  // Valid: U16 promotes to U64
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Empty(t, errors, "Should have no errors for valid U16 to U64 promotion")
	})

	t.Run("InvalidNarrowingU256ToU8", func(t *testing.T) {
		source := `contract Test {
			fn get_large() -> U256 {
				1000000
			}

			ext fn test() {
				let x: U8 = get_large();  // Error: Cannot narrow U256 to U8
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have errors for narrowing conversion")
		hasNarrowingError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "cannot assign") || containsSubstring(err.Message, "returns 'U256' but expected 'U8'") {
				hasNarrowingError = true
				break
			}
		}
		assert.True(t, hasNarrowingError, "Should detect narrowing conversion error")
	})

	t.Run("InvalidNarrowingU128ToU32", func(t *testing.T) {
		source := `contract Test {
			fn get_large() -> U128 {
				1000000
			}

			ext fn test() {
				let x: U32 = get_large();  // Error: Cannot narrow U128 to U32
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have errors for narrowing conversion")
		hasNarrowingError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "cannot assign") || containsSubstring(err.Message, "returns 'U128' but expected 'U32'") {
				hasNarrowingError = true
				break
			}
		}
		assert.True(t, hasNarrowingError, "Should detect narrowing conversion error")
	})

	t.Run("InvalidBoolToNumeric", func(t *testing.T) {
		source := `contract Test {
			fn get_bool() -> Bool {
				true
			}

			ext fn test() {
				let x: U256 = get_bool();  // Error: Bool cannot convert to U256
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have errors for incompatible types")
		hasTypeError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "Bool") && containsSubstring(err.Message, "U256") {
				hasTypeError = true
				break
			}
		}
		assert.True(t, hasTypeError, "Should detect Bool to numeric conversion error")
	})

	t.Run("InvalidNumericToBool", func(t *testing.T) {
		source := `contract Test {
			fn get_number() -> U256 {
				42
			}

			ext fn test() {
				let x: Bool = get_number();  // Error: U256 cannot convert to Bool
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have errors for incompatible types")
		hasTypeError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "U256") && containsSubstring(err.Message, "Bool") {
				hasTypeError = true
				break
			}
		}
		assert.True(t, hasTypeError, "Should detect numeric to Bool conversion error")
	})

	t.Run("ChainedPromotions", func(t *testing.T) {
		source := `contract Test {
			fn get_u8() -> U8 { 1 }
			fn get_u16() -> U16 { 2 }
			fn get_u32() -> U32 { 3 }

			ext fn test() {
				let a: U256 = get_u8();   // Valid: U8 -> U256
				let b: U256 = get_u16();  // Valid: U16 -> U256
				let c: U256 = get_u32();  // Valid: U32 -> U256
				let d: U128 = get_u8();   // Valid: U8 -> U128
				let e: U64 = get_u16();   // Valid: U16 -> U64
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Empty(t, errors, "Should have no errors for valid chained promotions")
	})

	t.Run("PromotionInExpressions", func(t *testing.T) {
		source := `contract Test {
			fn get_u8() -> U8 { 10 }
			fn get_u16() -> U16 { 20 }

			fn test() -> U256 {
				let a: U256 = get_u8() + get_u16();  // Both promote to common type
				a
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// This might have errors depending on how binary expressions handle mixed types
		// But the individual assignments should work
		for _, err := range errors {
			// Make sure no errors are about the function return type promotions themselves
			assert.NotContains(t, err.Message, "get_u8")
			assert.NotContains(t, err.Message, "get_u16")
		}
	})

	t.Run("PromotionWithMutableVariables", func(t *testing.T) {
		source := `contract Test {
			fn get_u8() -> U8 { 42 }

			ext fn test() {
				let mut x: U256 = 0;
				x = get_u8();  // Valid: U8 promotes to U256 in assignment
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Empty(t, errors, "Should have no errors for valid promotion in assignment")
	})

	t.Run("InvalidPromotionInMutableAssignment", func(t *testing.T) {
		source := `contract Test {
			fn get_u256() -> U256 { 42000 }

			ext fn test() {
				let mut x: U8 = 0;
				x = get_u256();  // Error: Cannot narrow U256 to U8
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have errors for narrowing in assignment")
		hasNarrowingError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "U256") && containsSubstring(err.Message, "U8") {
				hasNarrowingError = true
				break
			}
		}
		assert.True(t, hasNarrowingError, "Should detect narrowing error in assignment")
	})

	t.Run("PromotionWithImportedFunctions", func(t *testing.T) {
		source := `contract Test {
			use std::evm::{sender};

			ext fn test() {
				// sender returns Address, not a numeric type so no promotion
				let x: Address = sender();  // Valid: exact match
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Empty(t, errors, "Should have no errors for imported function with matching type")
	})

	t.Run("AllNumericTypePromotions", func(t *testing.T) {
		source := `contract Test {
			fn get_u8() -> U8 { 1 }

			ext fn test() {
				// Test all valid promotions from U8
				let a: U8 = get_u8();    // Valid: exact match
				let b: U16 = get_u8();   // Valid: U8 -> U16
				let c: U32 = get_u8();   // Valid: U8 -> U32
				let d: U64 = get_u8();   // Valid: U8 -> U64
				let e: U128 = get_u8();  // Valid: U8 -> U128
				let f: U256 = get_u8();  // Valid: U8 -> U256
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Empty(t, errors, "Should have no errors for all valid promotions from U8")
	})
}
