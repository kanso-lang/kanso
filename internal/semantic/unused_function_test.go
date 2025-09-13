package semantic

import (
	"kanso/internal/parser"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestUnusedFunctionDetection(t *testing.T) {
	t.Run("UnusedPrivateFunction", func(t *testing.T) {
		source := `contract Test {
			fn helper() -> U256 {  // This function is never used
				return 42;
			}

			ext fn main() -> U256 {
				return 100;  // Uses a literal, not the helper function
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.NotEmpty(t, errors, "Should have error for unused function")
		hasUnusedError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "never used") && containsSubstring(err.Message, "helper") {
				hasUnusedError = true
				break
			}
		}
		assert.True(t, hasUnusedError, "Should detect unused helper function")
	})

	t.Run("UsedPrivateFunction", func(t *testing.T) {
		source := `contract Test {
			fn helper() -> U256 {
				return 42;
			}

			ext fn main() -> U256 {
				return helper();  // This function is used
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Filter out flow analysis warnings
		typeErrors := []SemanticError{}
		for _, err := range errors {
			if !containsSubstring(err.Message, "unreachable code") && !containsSubstring(err.Message, "never used") {
				typeErrors = append(typeErrors, err)
			}
		}
		assert.Empty(t, typeErrors, "Should have no errors for used function")
	})

	t.Run("ExternalFunctionNotReportedAsUnused", func(t *testing.T) {
		source := `contract Test {
			ext fn public_function() -> U256 {  // External functions are entry points
				return 42;
			}

			fn unused_helper() -> U256 {  // This should be reported as unused
				return 100;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should report unused_helper but not public_function
		hasUnusedError := false
		hasExternalError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "never used") {
				if containsSubstring(err.Message, "unused_helper") {
					hasUnusedError = true
				}
				if containsSubstring(err.Message, "public_function") {
					hasExternalError = true
				}
			}
		}
		assert.True(t, hasUnusedError, "Should detect unused helper function")
		assert.False(t, hasExternalError, "Should not report external function as unused")
	})

	t.Run("ConstructorNotReportedAsUnused", func(t *testing.T) {
		source := `contract Test {
			#[create]
			fn create() {  // Constructor functions are entry points
				// Initialize contract
			}

			fn unused_helper() -> U256 {  // This should be reported as unused
				return 100;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should report unused_helper but not create
		hasUnusedError := false
		hasConstructorError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "never used") {
				if containsSubstring(err.Message, "unused_helper") {
					hasUnusedError = true
				}
				if containsSubstring(err.Message, "create") {
					hasConstructorError = true
				}
			}
		}
		assert.True(t, hasUnusedError, "Should detect unused helper function")
		assert.False(t, hasConstructorError, "Should not report constructor as unused")
	})

	t.Run("ChainedFunctionCalls", func(t *testing.T) {
		source := `contract Test {
			fn helper_a() -> U256 {
				return 42;
			}

			fn helper_b() -> U256 {
				return helper_a();  // Uses helper_a
			}

			fn unused_helper() -> U256 {  // This is never used
				return 100;
			}

			ext fn main() -> U256 {
				return helper_b();  // Uses helper_b, which uses helper_a
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should only report unused_helper
		hasUnusedError := false
		hasUsedError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "never used") {
				if containsSubstring(err.Message, "unused_helper") {
					hasUnusedError = true
				}
				if containsSubstring(err.Message, "helper_a") || containsSubstring(err.Message, "helper_b") {
					hasUsedError = true
				}
			}
		}
		assert.True(t, hasUnusedError, "Should detect unused helper function")
		assert.False(t, hasUsedError, "Should not report used functions as unused")
	})

	t.Run("MultipleUnusedFunctions", func(t *testing.T) {
		source := `contract Test {
			fn unused_a() -> U256 {  // Unused
				return 42;
			}

			fn unused_b() -> U256 {  // Unused
				return 100;
			}

			fn used_helper() -> U256 {  // Used
				return 200;
			}

			ext fn main() -> U256 {
				return used_helper();  // Only this chain is used
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should detect both unused functions
		unusedCount := 0
		for _, err := range errors {
			if containsSubstring(err.Message, "never used") {
				unusedCount++
			}
		}
		assert.Equal(t, 2, unusedCount, "Should detect exactly 2 unused functions")
	})

	t.Run("AllFunctionsUsed", func(t *testing.T) {
		source := `contract Test {
			fn helper() -> U256 {
				return 42;
			}

			ext fn main() -> U256 {
				return helper();
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Filter out flow analysis warnings
		unusedErrors := []SemanticError{}
		for _, err := range errors {
			if containsSubstring(err.Message, "never used") {
				unusedErrors = append(unusedErrors, err)
			}
		}
		assert.Empty(t, unusedErrors, "Should have no unused function errors")
	})
}
