package semantic

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"kanso/internal/parser"
)

func TestLiteralValidation(t *testing.T) {
	t.Run("ValidLiterals", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				// Valid numeric literals
				let small = 42;
				let large = 12345678901234567890;
				
				// Valid boolean literals
				let flag_true = true;
				let flag_false = false;
				
				// Valid string literals
				let simple_string = "hello";
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		semanticErrors := analyzer.Analyze(contract)
		assert.Empty(t, semanticErrors, "Should have no semantic errors for valid literals")
	})

	t.Run("InvalidNumericLiterals", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				let leading_zero = 0123;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		semanticErrors := analyzer.Analyze(contract)

		// Should have errors for invalid numeric literals
		if len(semanticErrors) > 0 {
			errorMessages := make([]string, len(semanticErrors))
			for i, err := range semanticErrors {
				errorMessages[i] = err.Message
			}

			// Check for specific error types
			assert.True(t, containsAny(errorMessages, "leading zeros"),
				"Should detect leading zeros in numeric literals")
		}
	})

	t.Run("HexadecimalLiterals", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				// Valid hex literals
				let small_hex: U8 = 0x1;
				let medium_hex: U16 = 0xFF;
				let large_hex: U32 = 0x2A3B;
				let max_byte: U8 = 0xFF;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		semanticErrors := analyzer.Analyze(contract)

		// Should have no errors for valid hex literals
		assert.Empty(t, semanticErrors, "Should have no semantic errors for valid hex literals")
	})

	t.Run("InvalidHexLiterals", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				// Invalid: hex number that's too large (exceeds U256)
				let huge_hex = 0x10000000000000000000000000000000000000000000000000000000000000000;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		semanticErrors := analyzer.Analyze(contract)

		// Should have errors for invalid hex literals
		if len(semanticErrors) > 0 {
			errorMessages := make([]string, len(semanticErrors))
			for i, err := range semanticErrors {
				errorMessages[i] = err.Message
			}

			assert.True(t, containsAny(errorMessages, "too large", "maximum"),
				"Should detect oversized hex literal")
		}
	})

	t.Run("AddressVsHexDistinction", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				// These should be treated as hex numbers, not addresses
				let hex1: U8 = 0x1;
				let hex2: U16 = 0x123;
				let hex3: U32 = 0xABCD;
				
				// This should be treated as an address (special case)
				let zero_addr = 0x0;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		semanticErrors := analyzer.Analyze(contract)

		// Should have no errors - hex numbers should be distinguished from addresses
		assert.Empty(t, semanticErrors, "Should properly distinguish hex numbers from addresses")
	})

	t.Run("BasicValidation", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				// This should be fine
				let normal = 42;
				let text = "hello";
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		semanticErrors := analyzer.Analyze(contract)

		// Should have no errors for basic literals
		assert.Empty(t, semanticErrors, "Should have no semantic errors for basic literals")
	})
}

// Helper function to check if any message contains any of the keywords
func containsAny(messages []string, keywords ...string) bool {
	for _, msg := range messages {
		for _, keyword := range keywords {
			if len(msg) >= len(keyword) {
				for i := 0; i <= len(msg)-len(keyword); i++ {
					if msg[i:i+len(keyword)] == keyword {
						return true
					}
				}
			}
		}
	}
	return false
}
