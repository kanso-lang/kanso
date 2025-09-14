package semantic

import (
	"testing"

	"kanso/internal/parser"

	"github.com/stretchr/testify/assert"
)

func TestUnusedVariableDetection(t *testing.T) {
	t.Run("UnusedVariable", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				let unused = 42;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have error for unused variable")
		hasUnusedError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "unused") && containsSubstring(err.Message, "never used") {
				hasUnusedError = true
				break
			}
		}
		assert.True(t, hasUnusedError, "Should detect unused variable")
	})

	t.Run("UsedVariable", func(t *testing.T) {
		source := `contract Test {
			ext fn test() -> U256 {
				let used = 42;
				let result = used + 10;
				return result;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Filter out other errors, we only care about unused variable errors
		unusedErrors := []SemanticError{}
		for _, err := range errors {
			if containsSubstring(err.Message, "never used") {
				unusedErrors = append(unusedErrors, err)
			}
		}
		assert.Empty(t, unusedErrors, "Should have no unused variable errors when variable is used")
	})

	t.Run("MultipleUnusedVariables", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				let unused1 = 42;
				let unused2 = "hello";
				let unused3 = true;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		unusedCount := 0
		for _, err := range errors {
			if containsSubstring(err.Message, "never used") {
				unusedCount++
			}
		}
		assert.Equal(t, 3, unusedCount, "Should detect all three unused variables")
	})

	t.Run("ParametersNotConsideredUnused", func(t *testing.T) {
		source := `contract Test {
			ext fn test(param1: U256, param2: Bool) {
				let unused = 42;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should only complain about 'unused', not about parameters
		unusedErrors := []string{}
		for _, err := range errors {
			if containsSubstring(err.Message, "never used") {
				unusedErrors = append(unusedErrors, err.Message)
			}
		}
		assert.Equal(t, 1, len(unusedErrors), "Should only detect the unused local variable, not parameters")
		assert.True(t, containsSubstring(unusedErrors[0], "unused"), "Error should be about 'unused' variable")
	})

	t.Run("VariableUsedInDifferentContexts", func(t *testing.T) {
		source := `contract Test {
			ext fn test() -> U256 {
				let x = 42;
				let y = x + 10;  // x used in expression
				return y;        // y used in return
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		unusedErrors := []SemanticError{}
		for _, err := range errors {
			if containsSubstring(err.Message, "never used") {
				unusedErrors = append(unusedErrors, err)
			}
		}
		assert.Empty(t, unusedErrors, "Should have no unused variable errors when variables are used in expressions and returns")
	})
}

func TestMutableVariableAnalysis(t *testing.T) {
	t.Run("MutableNeverModified", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				let mut never_modified = 42;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have error for mutable variable never modified")
		hasMutableError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "mutable") && containsSubstring(err.Message, "never modified") {
				hasMutableError = true
				break
			}
		}
		assert.True(t, hasMutableError, "Should detect mutable variable that's never modified")
	})

	t.Run("MutableProperlyUsed", func(t *testing.T) {
		source := `contract Test {
			ext fn test() -> U256 {
				let mut counter = 0;
				counter = 42;
				return counter;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Filter out other errors, check only mutable-related errors
		mutableErrors := []SemanticError{}
		for _, err := range errors {
			if containsSubstring(err.Message, "mutable") {
				mutableErrors = append(mutableErrors, err)
			}
		}
		assert.Empty(t, mutableErrors, "Should have no mutable variable errors when properly used")
	})

	t.Run("MutableModifiedButNotRead", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				let mut modified_unused = 42;
				modified_unused = 100;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have error for modified but unused variable")
		hasModifiedUnusedError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "modified") && containsSubstring(err.Message, "never used") {
				hasModifiedUnusedError = true
				break
			}
		}
		assert.True(t, hasModifiedUnusedError, "Should detect modified but unused variable")
	})

	t.Run("MutableMultipleAssignments", func(t *testing.T) {
		source := `contract Test {
			ext fn test() -> U256 {
				let mut accumulator = 0;
				accumulator += 10;
				accumulator *= 2;
				return accumulator;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Filter out other errors, check only mutable-related errors
		mutableErrors := []SemanticError{}
		for _, err := range errors {
			if containsSubstring(err.Message, "mutable") {
				mutableErrors = append(mutableErrors, err)
			}
		}
		assert.Empty(t, mutableErrors, "Should have no mutable variable errors when variable is modified and used")
	})

	t.Run("MutableUsedBeforeModified", func(t *testing.T) {
		source := `contract Test {
			ext fn test() -> U256 {
				let mut value = 42;
				let initial = value;  // Used before modification
				value = 100;
				return value + initial;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should be no errors - variable is used both before and after modification
		mutableErrors := []SemanticError{}
		for _, err := range errors {
			if containsSubstring(err.Message, "mutable") {
				mutableErrors = append(mutableErrors, err)
			}
		}
		assert.Empty(t, mutableErrors, "Should have no mutable variable errors when variable is used before and after modification")
	})

	t.Run("ImmutableVariableNoErrors", func(t *testing.T) {
		source := `contract Test {
			ext fn test() -> U256 {
				let immutable = 42;
				return immutable;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should have no mutability-related errors for immutable variables
		mutableErrors := []SemanticError{}
		for _, err := range errors {
			if containsSubstring(err.Message, "mutable") {
				mutableErrors = append(mutableErrors, err)
			}
		}
		assert.Empty(t, mutableErrors, "Should have no mutable variable errors for immutable variables")
	})
}

func TestComplexVariableUsageScenarios(t *testing.T) {
	t.Run("VariableUsedInFunctionCall", func(t *testing.T) {
		source := `contract Test {
			fn helper(x: U256) -> U256 {
				x * 2
			}

			ext fn test() -> U256 {
				let value = 42;
				return helper(value);
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		unusedErrors := []SemanticError{}
		for _, err := range errors {
			if containsSubstring(err.Message, "never used") {
				unusedErrors = append(unusedErrors, err)
			}
		}
		assert.Empty(t, unusedErrors, "Should have no unused variable errors when variable is used in function call")
	})

	t.Run("VariableUsedInCondition", func(t *testing.T) {
		source := `contract Test {
			ext fn test() -> Bool {
				let flag = true;
				let result = flag;
				return result;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		unusedErrors := []SemanticError{}
		for _, err := range errors {
			if containsSubstring(err.Message, "never used") {
				unusedErrors = append(unusedErrors, err)
			}
		}
		assert.Empty(t, unusedErrors, "Should have no unused variable errors when variable is used in condition")
	})

	t.Run("MixedUsagePatterns", func(t *testing.T) {
		source := `contract Test {
			ext fn test() -> U256 {
				let used = 10;
				let unused = 20;
				let mut mutable_used = 30;
				let mut mutable_unused = 40;
				let mut mutable_modified_unused = 50;
				
				mutable_used = 35;
				let result = used + mutable_used;
				mutable_modified_unused = 55;
				
				return result;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should detect multiple issues
		unusedErrors := 0
		mutableNeverModifiedErrors := 0
		modifiedUnusedErrors := 0

		for _, err := range errors {
			if containsSubstring(err.Message, "never used") {
				unusedErrors++
			}
			if containsSubstring(err.Message, "mutable") && containsSubstring(err.Message, "never modified") {
				mutableNeverModifiedErrors++
			}
			if containsSubstring(err.Message, "modified") && containsSubstring(err.Message, "never used") {
				modifiedUnusedErrors++
			}
		}

		assert.True(t, unusedErrors >= 2, "Should detect at least 2 unused variables")
		assert.Equal(t, 1, mutableNeverModifiedErrors, "Should detect 1 mutable variable never modified")
		assert.True(t, modifiedUnusedErrors >= 1, "Should detect at least 1 modified but unused variable")
	})

	t.Run("ReadThenModifyThenUnused", func(t *testing.T) {
		source := `contract Test {
			ext fn test() -> U256 {
				let mut value = 10;
				let first_use = value + 5;  // Variable used first
				value = 20;                 // Then modified
				// New value never used - should be error
				return first_use;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should detect that the modified value is never used
		hasModifiedUnusedError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "modified") && containsSubstring(err.Message, "never used") {
				hasModifiedUnusedError = true
				break
			}
		}
		assert.True(t, hasModifiedUnusedError, "Should detect that variable is modified but new value never used")
	})
}
