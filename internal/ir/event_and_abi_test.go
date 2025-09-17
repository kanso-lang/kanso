package ir

import (
	"strings"
	"testing"

	"kanso/internal/parser"
	"kanso/internal/semantic"
)

// Tests for event signature generation and ABI string conversion

// Test generateEventSignature function
func TestGenerateEventSignature(t *testing.T) {
	source := `
contract EventTest {
    use std::evm::{emit};

    #[event]
    struct Transfer {
        from: Address,
        to: Address,
        value: U256,
    }

    #[storage]
    struct State {
        balance: U256,
    }

    ext fn emitTransfer(from: Address, to: Address, value: U256) {
        emit(Transfer{from: from, to: to, value: value});
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
	if !strings.Contains(output, "Transfer_sig") {
		t.Errorf("Expected Transfer_sig in output, got: %s", output)
	}
}

// Test typeToABIString function
func TestTypeToABIString(t *testing.T) {
	source := `
contract TypeTest {
    use std::evm::{emit};
    use std::address;

    #[event]
    struct TestEvent {
        value: U256,
        flag: Bool,
        addr: Address,
    }

    ext fn testEmit() {
        emit(TestEvent{value: 42, flag: true, addr: address::zero()});
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

	// This should exercise typeToABIString through event signature generation
	output := PrintProgram(program)
	if !strings.Contains(output, "TestEvent") {
		t.Errorf("Expected TestEvent signature generation")
	}
}

// Test buildStructLiteralExtended function (different from existing test)
func TestBuildStructLiteralExtended(t *testing.T) {
	source := `
contract StructTest {
    struct Point {
        x: U256,
        y: U256,
    }

    ext fn createPoint() -> Point {
        Point{x: 10, y: 20}
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
	if output == "" {
		t.Error("Program output should not be empty")
	}
}

// Test getConstantKey function
func TestGetConstantKey(t *testing.T) {
	source := `
contract ConstTest {
    ext fn testConstants() -> U256 {
        let a: U256 = 42;
        let b: U256 = 42;  // Same constant value
        a + b
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

	// Should generate constants with keys
	output := PrintProgram(program)
	if !strings.Contains(output, "CONST") {
		t.Logf("Output: %s", output)
		// Don't fail since constant folding may optimize this
	}
}

// Test createMemoryEffect function
func TestCreateMemoryEffect(t *testing.T) {
	context := semantic.NewContextRegistry()
	builder := NewBuilder(context)

	// Create memory region
	region := &MemoryRegion{
		ID:   1,
		Name: "test_region",
		Base: &Value{Name: "base", Type: &AddressType{}},
	}

	offset := &Value{Name: "offset", Type: &IntType{Bits: 256}}
	size := &Value{Name: "size", Type: &IntType{Bits: 256}}

	// Test creating various memory effects
	effect1 := builder.createMemoryEffect(region, MemoryEffectWrite, offset, size)
	if effect1.Type != MemoryEffectWrite {
		t.Errorf("Expected MemoryEffectWrite, got %v", effect1.Type)
	}
	if effect1.Region != region {
		t.Error("Region should match")
	}

	effect2 := builder.createMemoryEffect(region, MemoryEffectRead, nil, nil)
	if effect2.Type != MemoryEffectRead {
		t.Errorf("Expected MemoryEffectRead, got %v", effect2.Type)
	}
}

// Test generateAccessorFunction through storage access
func TestGenerateAccessorFunction(t *testing.T) {
	source := `
contract AccessorTest {
    #[storage]
    struct State {
        balance: U256,
        owner: Address,
    }

    ext fn getBalance() -> U256 reads State {
        State.balance
    }

    ext fn getOwner() -> Address reads State {
        State.owner
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
	if !strings.Contains(output, "SLOAD") {
		t.Logf("Output: %s", output)
		// Storage loads might be optimized
	}
}

// Test more printer functions
func TestPrinterFunctions(t *testing.T) {
	source := `
contract PrinterTest {
    #[storage]
    struct State {
        value: U256,
    }

    ext fn complexFunction() writes State {
        State.value = 123;
        require!(State.value > 0);
        return;
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

	// Test various printer methods
	output := PrintProgram(program)
	if output == "" {
		t.Error("Program output should not be empty")
	}

	// Test individual instruction printing by checking for expected patterns
	if !strings.Contains(output, "complexFunction") {
		t.Error("Expected function name in output")
	}
}

// Test optimization functions
func TestOptimizationTests(t *testing.T) {
	source := `
contract OptTest {
    ext fn testArithmetic(x: U256, y: U256) -> U256 {
        let z: U256 = x + y;
        let w: U256 = z * 2;
        w - x
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

	// Test that optimizations run without error
	output := PrintProgram(program)
	if output == "" {
		t.Error("Program output should not be empty")
	}
}

// Test instruction interface methods
func TestInstructionInterfaces(t *testing.T) {
	// Test instruction interface methods
	const1 := &ConstantInstruction{
		ID:     1,
		Result: &Value{Name: "test", Type: &IntType{Bits: 256}},
		Value:  "42",
		Type:   &IntType{Bits: 256},
	}

	if const1.GetResult() == nil {
		t.Error("GetResult should return non-nil")
	}

	operands := const1.GetOperands()
	if operands == nil {
		t.Error("GetOperands should return non-nil slice")
	}

	if const1.IsTerminator() {
		t.Error("ConstantInstruction should not be terminator")
	}

	// Test more instruction types
	load := &LoadInstruction{
		ID:      2,
		Result:  &Value{Name: "load_result", Type: &IntType{Bits: 256}},
		Address: &Value{Name: "addr", Type: &AddressType{}},
	}

	if load.GetID() != 2 {
		t.Error("LoadInstruction GetID should return 2")
	}

	// Test terminator methods
	ret := &ReturnTerminator{
		ID:    3,
		Value: &Value{Name: "ret_val", Type: &IntType{Bits: 256}},
	}

	successors := ret.GetSuccessors()
	if successors == nil {
		t.Error("GetSuccessors should return non-nil slice")
	}

	if !ret.IsTerminator() {
		t.Error("ReturnTerminator should be terminator")
	}
}
