package semantic

import (
	"kanso/internal/parser"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestFunctionReturnTypeValidation(t *testing.T) {
	t.Run("NumericLiteralReturnedAsBool", func(t *testing.T) {
		source := `contract Test {
			fn get_bool() -> Bool {
				return 42;  // Error: returning numeric literal from Bool function
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.NotEmpty(t, errors, "Should have error for numeric literal returned as Bool")
		hasTypeError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "Bool") || containsSubstring(err.Message, "expected") {
				hasTypeError = true
				break
			}
		}
		assert.True(t, hasTypeError, "Should detect type mismatch in return")
	})

	t.Run("BoolReturnedAsNumeric", func(t *testing.T) {
		source := `contract Test {
			fn get_number() -> U256 {
				return true;  // Error: returning bool from numeric function
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.NotEmpty(t, errors, "Should have error for bool returned as numeric")
		hasTypeError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "U256") || containsSubstring(err.Message, "Bool") {
				hasTypeError = true
				break
			}
		}
		assert.True(t, hasTypeError, "Should detect type mismatch in return")
	})

	t.Run("ExplicitReturnTypeMismatch", func(t *testing.T) {
		source := `contract Test {
			fn get_u8() -> U8 {
				return 42;  // This should work
			}

			fn get_bool() -> Bool {
				return 42;  // Error: explicit return with wrong type
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.NotEmpty(t, errors, "Should have error for explicit return type mismatch")
		hasTypeError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "Bool") {
				hasTypeError = true
				break
			}
		}
		assert.True(t, hasTypeError, "Should detect explicit return type mismatch")
	})

	t.Run("TailExpressionTypeMismatch", func(t *testing.T) {
		source := `contract Test {
			fn get_bool() -> Bool {
				let x = 10;
				return x;  // Error: U256 but function returns Bool
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.NotEmpty(t, errors, "Should have error for tail expression type mismatch")
		hasTypeError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "Bool") || containsSubstring(err.Message, "U256") {
				hasTypeError = true
				break
			}
		}
		assert.True(t, hasTypeError, "Should detect tail expression type mismatch")
	})

	t.Run("ValidNumericPromotion", func(t *testing.T) {
		source := `contract Test {
			fn get_u256() -> U256 {
				let x: U8 = 42;
				return x;  // Valid: U8 promotes to U256
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Filter out flow analysis warnings and unused function warnings - we only care about type errors here
		typeErrors := []SemanticError{}
		for _, err := range errors {
			if !containsSubstring(err.Message, "unreachable code") && !containsSubstring(err.Message, "never used") {
				typeErrors = append(typeErrors, err)
			}
		}
		assert.Empty(t, typeErrors, "Should allow valid numeric promotion in return")
	})

	t.Run("InvalidNumericNarrowing", func(t *testing.T) {
		source := `contract Test {
			fn get_u8() -> U8 {
				let x: U256 = 42000;
				return x;  // Error: Cannot narrow U256 to U8
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.NotEmpty(t, errors, "Should have error for numeric narrowing in return")
		hasNarrowingError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "U256") && containsSubstring(err.Message, "U8") {
				hasNarrowingError = true
				break
			}
		}
		assert.True(t, hasNarrowingError, "Should detect narrowing in return")
	})

	t.Run("VoidFunctionWithReturn", func(t *testing.T) {
		source := `contract Test {
			fn void_func() {
				return 42;  // Error: void function returning value
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.NotEmpty(t, errors, "Should have error for void function returning value")
		hasReturnError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "void") || containsSubstring(err.Message, "return") {
				hasReturnError = true
				break
			}
		}
		assert.True(t, hasReturnError, "Should detect void function returning value")
	})

	t.Run("NonVoidFunctionReturningVoid", func(t *testing.T) {
		source := `contract Test {
			fn void_func() {
				// void function
			}

			fn get_number() -> U256 {
				return void_func();  // Error: returning void from non-void function
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.NotEmpty(t, errors, "Should have error for returning void from non-void function")
		hasVoidError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "void") || containsSubstring(err.Message, "does not return") {
				hasVoidError = true
				break
			}
		}
		assert.True(t, hasVoidError, "Should detect returning void from non-void function")
	})

	t.Run("ComplexExpressionReturn", func(t *testing.T) {
		source := `contract Test {
			fn get_bool() -> Bool {
				let a = 10;
				let b = 20;
				return a + b;  // Error: arithmetic expression (U256) returned as Bool
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.NotEmpty(t, errors, "Should have error for expression type mismatch")
		hasTypeError := false
		for _, err := range errors {
			// Check for any numeric type (U8, U256, etc.) being returned as Bool
			if (containsSubstring(err.Message, "Bool") && containsSubstring(err.Message, "U")) ||
				containsSubstring(err.Message, "type mismatch") ||
				containsSubstring(err.Message, "cannot return") {
				hasTypeError = true
				break
			}
		}
		assert.True(t, hasTypeError, "Should detect expression type mismatch in return")
	})

	t.Run("StringReturnTypeMismatch", func(t *testing.T) {
		source := `contract Test {
			use std::ascii::{String};

			fn get_string() -> String {
				return 42;  // Error: returning numeric when String expected
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.NotEmpty(t, errors, "Should have error for numeric returned as String")
		hasTypeError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "String") || containsSubstring(err.Message, "expected") {
				hasTypeError = true
				break
			}
		}
		assert.True(t, hasTypeError, "Should detect String type mismatch")
	})

	t.Run("AddressReturnTypeMismatch", func(t *testing.T) {
		source := `contract Test {
			fn get_address() -> Address {
				return true;  // Error: returning bool when Address expected
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.NotEmpty(t, errors, "Should have error for bool returned as Address")
		hasTypeError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "Address") || containsSubstring(err.Message, "Bool") {
				hasTypeError = true
				break
			}
		}
		assert.True(t, hasTypeError, "Should detect Address type mismatch")
	})

	t.Run("ValidBoolReturn", func(t *testing.T) {
		source := `contract Test {
			fn get_bool() -> Bool {
				return true;  // Valid: returning bool literal
			}

			fn get_bool2() -> Bool {
				let x = false;
				return x;  // Valid: returning bool variable
			}

			fn get_bool3() -> Bool {
				return false;  // Valid: explicit return
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Filter out flow analysis warnings and unused function warnings - we only care about type errors here
		typeErrors := []SemanticError{}
		for _, err := range errors {
			if !containsSubstring(err.Message, "unreachable code") && !containsSubstring(err.Message, "never used") {
				typeErrors = append(typeErrors, err)
			}
		}
		assert.Empty(t, typeErrors, "Should have no errors for valid bool returns")
	})
}
