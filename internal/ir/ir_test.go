package ir

import (
	"strings"
	"testing"

	"kanso/internal/ast"
	"kanso/internal/parser"
	"kanso/internal/semantic"
)

// Helper function to parse and analyze a contract for testing
func parseAndAnalyzeContract(t *testing.T, source string) (*ast.Contract, *semantic.ContextRegistry) {
	contract, parseErrors, scanErrors := parser.ParseSource("test.ka", source)
	if len(parseErrors) > 0 || len(scanErrors) > 0 {
		t.Fatalf("Parse errors: %v, Scan errors: %v", parseErrors, scanErrors)
	}
	if contract == nil {
		t.Fatal("Contract is nil")
	}

	analyzer := semantic.NewAnalyzer()
	errors := analyzer.Analyze(contract)
	context := analyzer.GetContext()
	if len(errors) > 0 {
		t.Fatalf("Semantic errors: %v", errors)
	}

	return contract, context
}

func TestBuildProgram_SimpleContract(t *testing.T) {
	source := `
contract SimpleTest {
    #[storage]
    struct State {
        value: U256,
    }

    #[create]
    fn create() writes State {
        State.value = 42;
    }

    ext fn getValue() -> U256 reads State {
        State.value
    }
}`

	contract, context := parseAndAnalyzeContract(t, source)

	program := BuildProgram(contract, context)

	if program == nil {
		t.Fatal("Program should not be nil")
	}

	if len(program.Functions) == 0 {
		t.Fatal("Program should have functions")
	}

	// Check that we have the expected functions
	functionNames := make(map[string]bool)
	for _, fn := range program.Functions {
		functionNames[fn.Name] = true
	}

	expectedFunctions := []string{"create", "getValue"}
	for _, expected := range expectedFunctions {
		if !functionNames[expected] {
			t.Errorf("Expected function %s not found", expected)
		}
	}
}

func TestBuildProgram_WithConstants(t *testing.T) {
	source := `
contract ConstTest {
    #[storage]
    struct State {
        flag: Bool,
    }

    #[create]
    fn create() writes State {
        State.flag = true;
    }
}`

	contract, context := parseAndAnalyzeContract(t, source)

	program := BuildProgram(contract, context)

	if program == nil {
		t.Fatal("Program should not be nil")
	}

	// Check that constants are created
	if len(program.Constants) == 0 {
		t.Fatal("Program should have constants")
	}

	// Verify canonical constants exist
	hasTrue := false
	hasTrueValue := false
	for _, constant := range program.Constants {
		if constant.Data == "true" || constant.Data == "1" {
			hasTrue = true
		}
		if constant.Value != nil && strings.Contains(constant.Value.Name, "true") {
			hasTrueValue = true
		}
	}

	if !hasTrue && !hasTrueValue {
		// Debug output
		t.Logf("Available constants: %d", len(program.Constants))
		for i, constant := range program.Constants {
			t.Logf("Constant %d: Data=%s, Value.Name=%s", i, constant.Data, constant.Value.Name)
		}
		t.Error("Expected canonical true constant not found")
	}
}

func TestPrintProgram(t *testing.T) {
	source := `
contract PrintTest {
    #[storage]
    struct State {
        counter: U256,
    }

    #[create]
    fn create() writes State {
        State.counter = 0;
    }

    ext fn increment() writes State {
        State.counter += 1;
    }
}`

	contract, context := parseAndAnalyzeContract(t, source)

	program := BuildProgram(contract, context)
	output := PrintProgram(program)

	if output == "" {
		t.Fatal("PrintProgram should return non-empty string")
	}

	// Check that output contains expected sections
	expectedSections := []string{
		"CONSTANTS:",
		"FUNCTION create",
		"FUNCTION increment",
		"CONTROL FLOW GRAPH:",
	}

	for _, section := range expectedSections {
		if !strings.Contains(output, section) {
			t.Errorf("Output should contain section: %s", section)
		}
	}
}

func TestConstEvalIntrinsics(t *testing.T) {
	source := `
contract IntrinsicTest {
    use std::address;

    #[storage]
    struct State {
        zero_addr: Address,
    }

    #[create]
    fn create() writes State {
        State.zero_addr = address::zero();
    }
}`

	contract, context := parseAndAnalyzeContract(t, source)

	program := BuildProgram(contract, context)
	output := PrintProgram(program)

	// Verify that address::zero() was const-eval'd to use %zero_addr
	if !strings.Contains(output, "%zero_addr = 0x0000000000000000000000000000000000000000") {
		t.Error("Expected zero_addr constant not found")
	}

	// Should not contain a call to address::zero
	if strings.Contains(output, "call") && strings.Contains(output, "zero") {
		t.Error("address::zero() should be const-eval'd, not called")
	}
}

func TestCompoundAssignmentNaming(t *testing.T) {
	source := `
contract CompoundTest {
    #[storage]
    struct State {
        balance: U256,
    }

    ext fn addToBalance(amount: U256) writes State {
        State.balance += amount;
    }
}`

	contract, context := parseAndAnalyzeContract(t, source)

	program := BuildProgram(contract, context)
	output := PrintProgram(program)

	// Check for descriptive naming instead of compound_result_*
	if strings.Contains(output, "compound_result") {
		t.Error("Should use descriptive naming, not compound_result_*")
	}

	// Should have new_balance naming
	if !strings.Contains(output, "new_balance") {
		t.Error("Expected descriptive new_balance naming")
	}
}

func TestMemoryEffects(t *testing.T) {
	source := `
contract EffectTest {
    use std::evm::{emit};

    #[event]
    struct TestEvent {
        value: U256,
    }

    ext fn emitEvent(value: U256) {
        emit(TestEvent{value});
    }
}`

	contract, context := parseAndAnalyzeContract(t, source)

	program := BuildProgram(contract, context)
	output := PrintProgram(program)

	// Check for memory effects on ABI encoding or LOG operations
	hasMemoryEffect := strings.Contains(output, "write(Memory)") ||
		strings.Contains(output, "read(Memory)") ||
		strings.Contains(output, "emits(Log)") ||
		strings.Contains(output, "ABI_ENC") ||
		strings.Contains(output, "LOG")

	if !hasMemoryEffect {
		t.Errorf("Expected memory effects or LOG operations in output, got: %s", output)
	}
}

func TestPerFunctionCFG(t *testing.T) {
	source := `
contract CFGTest {
    #[storage]
    struct State {
        value: U256,
    }

    #[create]
    fn create() writes State {
        State.value = 1;
    }

    ext fn getValue() -> U256 reads State {
        State.value
    }

    ext fn setValue(newValue: U256) writes State {
        State.value = newValue;
    }
}`

	contract, context := parseAndAnalyzeContract(t, source)

	program := BuildProgram(contract, context)
	output := PrintProgram(program)

	// Check for per-function CFG display
	expectedFunctionSections := []string{
		"Function: create",
		"Function: getValue",
		"Function: setValue",
	}

	for _, section := range expectedFunctionSections {
		if !strings.Contains(output, section) {
			t.Errorf("Expected per-function CFG section: %s", section)
		}
	}

	// Check for CFG structure elements
	cfgElements := []string{"Entry:", "Blocks:"}
	for _, element := range cfgElements {
		if !strings.Contains(output, element) {
			t.Errorf("Expected CFG element: %s", element)
		}
	}
}
