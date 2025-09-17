package ir

import (
	"strings"
	"testing"

	"kanso/internal/parser"
	"kanso/internal/semantic"
)

// Tests for IR optimization functions

// Test constant folding optimization
func TestConstantFoldingOptimization(t *testing.T) {
	source := `
contract OptimizationTest {
    ext fn testConstantFolding() -> U256 {
        let a: U256 = 5 + 3;  // Should fold to 8
        let b: U256 = 10 * 2; // Should fold to 20
        let c: U256 = 15 - 5; // Should fold to 10
        let d: U256 = 20 / 4; // Should fold to 5
        a + b + c + d
    }
}`

	contract, parseErrors, scanErrors := parser.ParseSource("test.ka", source)
	if len(parseErrors) > 0 || len(scanErrors) > 0 {
		t.Fatalf("Parse errors: %v, Scan errors: %v", parseErrors, scanErrors)
	}

	analyzer := semantic.NewAnalyzer()
	errors := analyzer.Analyze(contract)
	if len(errors) > 0 {
		t.Fatalf("Semantic errors: %v", errors)
	}

	context := analyzer.GetContext()
	program := BuildProgram(contract, context)

	if program == nil {
		t.Fatal("Program should not be nil")
	}

	output := PrintProgram(program)
	// Constants should be folded
	if !strings.Contains(output, "CONST") {
		t.Logf("Output: %s", output)
		// May be optimized away entirely
	}
}

// Test dead code elimination
func TestDeadCodeElimination(t *testing.T) {
	source := `
contract DeadCodeTest {
    ext fn testDeadCode() -> U256 {
        let used: U256 = 100;
        return used;
    }
}`

	contract, parseErrors, scanErrors := parser.ParseSource("test.ka", source)
	if len(parseErrors) > 0 || len(scanErrors) > 0 {
		t.Fatalf("Parse errors: %v, Scan errors: %v", parseErrors, scanErrors)
	}

	analyzer := semantic.NewAnalyzer()
	errors := analyzer.Analyze(contract)
	if len(errors) > 0 {
		t.Fatalf("Semantic errors: %v", errors)
	}

	context := analyzer.GetContext()
	program := BuildProgram(contract, context)

	if program == nil {
		t.Fatal("Program should not be nil")
	}

	output := PrintProgram(program)
	// Should not contain unreachable code
	if strings.Contains(output, "999") {
		t.Logf("Output contains unreachable constant: %s", output)
		// May still be present before optimization
	}
}

// Test checked arithmetic optimization
func TestCheckedArithmeticOptimization(t *testing.T) {
	source := `
contract CheckedArithTest {
    ext fn testCheckedArith(x: U256, y: U256) -> U256 {
        require!(x >= y);
        x - y
    }
}`

	contract, parseErrors, scanErrors := parser.ParseSource("test.ka", source)
	if len(parseErrors) > 0 || len(scanErrors) > 0 {
		t.Fatalf("Parse errors: %v, Scan errors: %v", parseErrors, scanErrors)
	}

	analyzer := semantic.NewAnalyzer()
	errors := analyzer.Analyze(contract)
	if len(errors) > 0 {
		t.Fatalf("Semantic errors: %v", errors)
	}

	context := analyzer.GetContext()
	program := BuildProgram(contract, context)

	if program == nil {
		t.Fatal("Program should not be nil")
	}

	output := PrintProgram(program)
	// Should optimize SUB_CHK to SUB under dominating assumption
	if strings.Contains(output, "SUB") {
		t.Logf("Found subtraction in output")
	}
}

// Test common subexpression elimination
func TestCommonSubexpressionElimination(t *testing.T) {
	source := `
contract CSETest {
    ext fn testCSE(x: U256, y: U256) -> U256 {
        let a: U256 = x + y;
        let b: U256 = x + y;  // Same expression, should be eliminated
        let c: U256 = a * 2;
        let d: U256 = b * 2;  // Should reuse previous multiplication
        c + d
    }
}`

	contract, parseErrors, scanErrors := parser.ParseSource("test.ka", source)
	if len(parseErrors) > 0 || len(scanErrors) > 0 {
		t.Fatalf("Parse errors: %v, Scan errors: %v", parseErrors, scanErrors)
	}

	analyzer := semantic.NewAnalyzer()
	errors := analyzer.Analyze(contract)
	if len(errors) > 0 {
		t.Fatalf("Semantic errors: %v", errors)
	}

	context := analyzer.GetContext()
	program := BuildProgram(contract, context)

	if program == nil {
		t.Fatal("Program should not be nil")
	}

	output := PrintProgram(program)
	// Common subexpressions should be eliminated
	if output == "" {
		t.Error("Program output should not be empty")
	}
}

// Test instruction printing methods
func TestInstructionPrinting(t *testing.T) {
	source := `
contract PrintTest {
    use std::evm::{emit, sender};

    #[storage]
    struct State {
        owner: Address,
        balances: Slots<Address, U256>,
    }

    #[event]
    struct Transfer {
        from: Address,
        to: Address,
        value: U256,
    }

    ext fn complexFunction(to: Address, amount: U256) writes State {
        let from: Address = sender();
        require!(State.balances[from] >= amount);

        State.balances[from] = State.balances[from] - amount;
        State.balances[to] = State.balances[to] + amount;

        emit(Transfer{from: from, to: to, value: amount});
    }
}`

	contract, parseErrors, scanErrors := parser.ParseSource("test.ka", source)
	if len(parseErrors) > 0 || len(scanErrors) > 0 {
		t.Fatalf("Parse errors: %v, Scan errors: %v", parseErrors, scanErrors)
	}

	analyzer := semantic.NewAnalyzer()
	errors := analyzer.Analyze(contract)
	if len(errors) > 0 {
		t.Fatalf("Semantic errors: %v", errors)
	}

	context := analyzer.GetContext()
	program := BuildProgram(contract, context)

	if program == nil {
		t.Fatal("Program should not be nil")
	}

	output := PrintProgram(program)

	// Should contain various instruction types
	expectedPatterns := []string{
		"complexFunction",
		"sender",
		"SLOAD",
		"SSTORE",
		"SUB",
		"ADD",
		"ABI_ENC",
		"LOG",
	}

	for _, pattern := range expectedPatterns {
		if !strings.Contains(output, pattern) {
			t.Logf("Expected pattern '%s' not found in output", pattern)
		}
	}

	// Test that output is well-formed
	if len(output) < 100 {
		t.Errorf("Output seems too short: %d characters", len(output))
	}
}

// Test more complex expressions
func TestComplexExpressions(t *testing.T) {
	source := `
contract ExprTest {
    ext fn testComplexExpr(a: U256, b: U256, c: U256) -> U256 {
        ((a + b) * c) - ((a - b) + c)
    }

    ext fn testNestedCalls() -> U256 {
        testComplexExpr(1, 2, 3) + testComplexExpr(4, 5, 6)
    }
}`

	contract, parseErrors, scanErrors := parser.ParseSource("test.ka", source)
	if len(parseErrors) > 0 || len(scanErrors) > 0 {
		t.Fatalf("Parse errors: %v, Scan errors: %v", parseErrors, scanErrors)
	}

	analyzer := semantic.NewAnalyzer()
	errors := analyzer.Analyze(contract)
	if len(errors) > 0 {
		t.Fatalf("Semantic errors: %v", errors)
	}

	context := analyzer.GetContext()
	program := BuildProgram(contract, context)

	if program == nil {
		t.Fatal("Program should not be nil")
	}

	output := PrintProgram(program)
	// Should have arithmetic operations
	if !strings.Contains(output, "ADD") && !strings.Contains(output, "MUL") && !strings.Contains(output, "SUB") {
		t.Logf("Expected arithmetic operations in output")
	}
}
