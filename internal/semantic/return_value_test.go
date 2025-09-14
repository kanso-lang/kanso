package semantic

import (
	"kanso/internal/parser"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestReturnValueValidation(t *testing.T) {
	t.Run("TypeMismatchInAssignment", func(t *testing.T) {
		source := `contract Test {
			fn returns_bool() -> Bool {
				true
			}

			ext fn test() {
				let x: U256 = returns_bool();  // Error: Bool vs U256 mismatch
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have at least one type mismatch error")
		hasReturnTypeError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "returns 'Bool' but expected 'U256'") {
				hasReturnTypeError = true
				break
			}
		}
		assert.True(t, hasReturnTypeError, "Should detect return type mismatch")
	})

	t.Run("TypeMismatchWithLocalFunction", func(t *testing.T) {
		source := `contract Test {
			fn get_number() -> U256 {
				42
			}

			ext fn test() {
				let flag: Bool = get_number();  // Error: U256 vs Bool mismatch
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have at least one type mismatch error")
		hasReturnTypeError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "returns 'U256' but expected 'Bool'") {
				hasReturnTypeError = true
				break
			}
		}
		assert.True(t, hasReturnTypeError, "Should detect return type mismatch")
	})
	t.Run("VoidFunctionUsedAsValue", func(t *testing.T) {
		source := `contract Test {
			fn void_func() {
				// Does nothing
			}

			ext fn test() {
				let x = void_func();  // Error: void function used as value
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Len(t, errors, 1, "Should have one error")
		assert.Contains(t, errors[0].Message, "does not return a value")
		assert.Contains(t, errors[0].Message, "void_func")
	})

	t.Run("VoidFunctionInReturn", func(t *testing.T) {
		source := `contract Test {
			fn void_func() {
				// Does nothing
			}

			ext fn test() -> U256 {
				return void_func();  // Error: void function used in return
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have at least one error")
		hasReturnError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "does not return a value") {
				hasReturnError = true
				break
			}
		}
		assert.True(t, hasReturnError, "Should detect void function in return context")
	})

	t.Run("VoidFunctionInAssignment", func(t *testing.T) {
		source := `contract Test {
			fn void_func() {
				// Does nothing
			}

			ext fn test() {
				let mut x: U256 = 0;
				x = void_func();  // Error: void function used in assignment
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have at least one error")
		hasAssignError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "does not return a value") {
				hasAssignError = true
				break
			}
		}
		assert.True(t, hasAssignError, "Should detect void function in assignment")
	})

	t.Run("ValidReturnValueUsage", func(t *testing.T) {
		source := `contract Test {
			fn returns_value() -> U256 {
				100
			}

			ext fn test() {
				let x = returns_value();  // Valid: function returns U256
				let mut y: U256 = returns_value();  // Valid
				y = returns_value();  // Valid
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused and mutable variable errors
		errors := FilterAllUnusedErrors(allErrors)

		assert.Empty(t, errors, "Should have no errors for valid return value usage")
	})

	t.Run("VoidFunctionAsStatement", func(t *testing.T) {
		source := `contract Test {
			fn void_func() {
				// Does nothing
			}

			ext fn test() {
				void_func();  // Valid: void function called as statement
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.Empty(t, errors, "Should have no errors when void function used as statement")
	})

	t.Run("ReturnValueIgnored", func(t *testing.T) {
		source := `contract Test {
			fn returns_value() -> U256 {
				100
			}

			ext fn test() {
				returns_value();  // Valid for now: ignoring return value (could be warning)
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// For now, we don't error on ignored return values
		assert.Empty(t, errors, "Should allow ignoring return values (for now)")
	})

	t.Run("ImportedFunctionReturnValue", func(t *testing.T) {
		source := `contract Test {
			use std::evm::{sender};

			ext fn test() {
				let addr = sender();  // Valid: sender returns Address
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Empty(t, errors, "Should handle imported function return values")
	})

	t.Run("ComplexReturnValueChain", func(t *testing.T) {
		source := `contract Test {
			fn get_value() -> U256 {
				100
			}

			fn process(x: U256) -> U256 {
				x * 2
			}

			ext fn test() -> U256 {
				let a = get_value();  // Valid
				let b = process(a);   // Valid
				let c = process(get_value());  // Valid: nested calls
				
				b + c
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.Empty(t, errors, "Should handle complex return value chains")
	})

	t.Run("MultipleFunctionCallsInExpression", func(t *testing.T) {
		source := `contract Test {
			fn get_a() -> U256 { 10 }
			fn get_b() -> U256 { 20 }

			ext fn test() -> U256 {
				let result = get_a() + get_b();  // Valid: both return values
				
				result
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.Empty(t, errors, "Should handle multiple function calls in expressions")
	})
}
