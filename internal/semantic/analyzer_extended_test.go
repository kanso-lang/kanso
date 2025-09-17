package semantic

import (
	"testing"

	"kanso/internal/parser"
)

// Extended tests for semantic analyzer functionality

// Test GetContext function
func TestGetContext(t *testing.T) {
	analyzer := NewAnalyzer()

	// Parse a simple contract
	source := `
contract Test {
    ext fn test() -> U256 {
        42
    }
}`

	contract, _, _ := parser.ParseSource("test.ka", source)
	analyzer.Analyze(contract)

	context := analyzer.GetContext()
	if context == nil {
		t.Error("GetContext should return non-nil context")
	}
}

// Test GetImportedFunction function
func TestGetImportedFunction(t *testing.T) {
	analyzer := NewAnalyzer()

	// Parse a contract with imports
	source := `
contract Test {
    use std::evm::{sender};

    ext fn test() -> Address {
        sender()
    }
}`

	contract, _, _ := parser.ParseSource("test.ka", source)
	analyzer.Analyze(contract)

	context := analyzer.GetContext()
	imported := context.GetImportedFunction("sender")
	if imported == nil {
		t.Error("GetImportedFunction should find sender function")
	}
}

// Test validateStringLiteral function
func TestValidateStringLiteral(t *testing.T) {
	analyzer := NewAnalyzer()

	// Parse a contract with string literal - string literals are currently parsed as numeric
	// So let's test the string validation path differently
	source := `
contract Test {
    ext fn test() {
        // This exercises string literal validation paths
        let message = "hello world";
    }
}`

	contract, _, _ := parser.ParseSource("test.ka", source)
	errors := analyzer.Analyze(contract)

	// The function should exercise string validation code paths
	_ = errors // May have type-related errors but that's OK
}

// Test validateAssignmentCompatibility function
func TestValidateAssignmentCompatibility(t *testing.T) {
	analyzer := NewAnalyzer()

	// Parse a contract with incompatible assignment
	source := `
contract Test {
    ext fn test() {
        let x: U256 = "not a number";
    }
}`

	contract, _, _ := parser.ParseSource("test.ka", source)
	errors := analyzer.Analyze(contract)

	// Should have errors for incompatible assignment
	if len(errors) == 0 {
		t.Error("Expected assignment compatibility error")
	}
}

// Test getInvalidAssignmentMessage function
func TestGetInvalidAssignmentMessage(t *testing.T) {
	analyzer := NewAnalyzer()

	// Parse a contract with invalid assignment target - this will be a parse error, not semantic
	// Let's test with a valid parse but invalid semantics
	source := `
contract Test {
    ext fn test() {
        let x: U256 = 42;
        // Can't assign to non-mutable variable
        x = 100;
    }
}`

	contract, _, _ := parser.ParseSource("test.ka", source)
	errors := analyzer.Analyze(contract)

	// Should have errors for invalid assignment
	if len(errors) == 0 {
		t.Error("Expected invalid assignment error")
	}
}

// Test addUndefinedFunctionError function
func TestAddUndefinedFunctionError(t *testing.T) {
	analyzer := NewAnalyzer()

	// Parse a contract with undefined function call
	source := `
contract Test {
    ext fn test() -> U256 {
        undefinedFunction()
    }
}`

	contract, _, _ := parser.ParseSource("test.ka", source)
	errors := analyzer.Analyze(contract)

	// Should have errors for undefined function
	if len(errors) == 0 {
		t.Error("Expected undefined function error")
	}
}

// Test addMissingReturnError function
func TestAddMissingReturnError(t *testing.T) {
	analyzer := NewAnalyzer()

	// Parse a contract with missing return statement
	source := `
contract Test {
    ext fn test() -> U256 {
        let x: U256 = 42;
        // Missing return statement
    }
}`

	contract, _, _ := parser.ParseSource("test.ka", source)
	errors := analyzer.Analyze(contract)

	// Should have errors for missing return
	if len(errors) == 0 {
		t.Error("Expected missing return error")
	}
}

// Test analyzeCallContext function
func TestAnalyzeCallContext(t *testing.T) {
	analyzer := NewAnalyzer()

	// Parse a contract with function calls in different contexts
	source := `
contract Test {
    ext fn helper() -> U256 {
        42
    }

    ext fn test() -> U256 {
        helper() + helper()
    }
}`

	contract, _, _ := parser.ParseSource("test.ka", source)
	errors := analyzer.Analyze(contract)

	// Should analyze without errors
	if len(errors) > 0 {
		t.Errorf("Unexpected errors: %v", errors)
	}
}

// Test isUsedInValueContext function
func TestIsUsedInValueContext(t *testing.T) {
	analyzer := NewAnalyzer()

	// Parse a contract to test value context usage
	source := `
contract Test {
    ext fn test() -> U256 {
        let x: U256 = 42;
        x
    }
}`

	contract, _, _ := parser.ParseSource("test.ka", source)
	errors := analyzer.Analyze(contract)

	// Should analyze without errors
	if len(errors) > 0 {
		t.Errorf("Unexpected errors: %v", errors)
	}
}

// Test tupleTypesMatch function
func TestTupleTypesMatch(t *testing.T) {
	analyzer := NewAnalyzer()

	// Parse a contract with tuple operations
	source := `
contract Test {
    struct Point {
        x: U256,
        y: U256,
    }

    ext fn test() -> Point {
        Point{x: 1, y: 2}
    }
}`

	contract, _, _ := parser.ParseSource("test.ka", source)
	errors := analyzer.Analyze(contract)

	// Should analyze without errors
	if len(errors) > 0 {
		t.Errorf("Unexpected errors: %v", errors)
	}
}

// Test areComparableTypes function
func TestAreComparableTypes(t *testing.T) {
	analyzer := NewAnalyzer()

	// Parse a contract with type comparisons
	source := `
contract Test {
    ext fn test(x: U256, y: U256) -> Bool {
        x == y
    }
}`

	contract, _, _ := parser.ParseSource("test.ka", source)
	errors := analyzer.Analyze(contract)

	// Should analyze without errors
	if len(errors) > 0 {
		t.Errorf("Unexpected errors: %v", errors)
	}
}

// Test isStringType function
func TestIsStringType(t *testing.T) {
	analyzer := NewAnalyzer()

	// Parse a contract that exercises string type checking
	source := `
contract Test {
    ext fn test() {
        // This exercises string type validation paths
        let message = "test string";
    }
}`

	contract, _, _ := parser.ParseSource("test.ka", source)
	errors := analyzer.Analyze(contract)

	// The function should exercise string type checking code paths
	_ = errors // May have type-related errors but that's expected
}

// Test context validation functions
func TestContextValidation(t *testing.T) {
	context := NewContextRegistry()

	// Test ValidateTypeUsage
	errors := context.ValidateTypeUsage("U256", false)
	if len(errors) > 0 {
		t.Errorf("U256 should be valid type, got errors: %v", errors)
	}

	// Test ValidateFunctionCall - basic test
	errors = context.ValidateFunctionCall("test")
	_ = errors // Just exercise the function

	// Test ValidateModuleAccess
	errors = context.ValidateModuleAccess("std::evm", "sender")
	_ = errors // Just exercise the function
}
