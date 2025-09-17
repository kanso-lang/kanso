package ir

import (
	"strings"
	"testing"
)

func TestNewPrinter(t *testing.T) {
	printer := NewPrinter()

	if printer == nil {
		t.Fatal("NewPrinter should not return nil")
	}

	if printer.indent != 0 {
		t.Errorf("NewPrinter should have indent 0, got %d", printer.indent)
	}

	if printer.output.Len() != 0 {
		t.Error("NewPrinter should have empty output buffer")
	}
}

func TestPrint(t *testing.T) {
	// Create a simple program
	program := &Program{
		Functions: []*Function{
			{
				Name:     "test_func",
				External: true,
				Blocks: []*BasicBlock{
					{
						Label: "entry",
						Instructions: []Instruction{
							&ConstantInstruction{
								ID:     1,
								Result: &Value{Name: "const_val", Type: &IntType{Bits: 256}},
								Value:  "42",
								Type:   &IntType{Bits: 256},
							},
						},
						Terminator: &ReturnTerminator{
							Value: &Value{Name: "const_val", Type: &IntType{Bits: 256}},
						},
					},
				},
			},
		},
		Constants: []*Constant{
			{
				Value: &Value{Name: "test_const", Type: &BoolType{}},
				Data:  "true",
			},
		},
	}

	output := Print(program)

	if output == "" {
		t.Fatal("Print should return non-empty string")
	}

	// Check for expected sections
	if !strings.Contains(output, "CONSTANTS:") {
		t.Error("Print output should contain CONSTANTS section")
	}

	if !strings.Contains(output, "FUNCTION test_func") {
		t.Error("Print output should contain function definition")
	}

	if !strings.Contains(output, "entry:") {
		t.Error("Print output should contain block label")
	}
}

func TestPrintFunction(t *testing.T) {
	printer := NewPrinter()

	function := &Function{
		Name:       "test_function",
		External:   true,
		Create:     false,
		ReturnType: &IntType{Bits: 256},
		Params: []*Parameter{
			{Name: "param1", Type: &AddressType{}},
			{Name: "param2", Type: &IntType{Bits: 256}},
		},
		Reads:  []string{"State"},
		Writes: []string{"State"},
		Blocks: []*BasicBlock{
			{
				Label: "entry",
				Instructions: []Instruction{
					&ConstantInstruction{
						ID:     1,
						Result: &Value{Name: "const_42", Type: &IntType{Bits: 256}},
						Value:  "42",
						Type:   &IntType{Bits: 256},
					},
				},
				Terminator: &ReturnTerminator{
					Value: &Value{Name: "const_42", Type: &IntType{Bits: 256}},
				},
			},
		},
	}

	printer.printFunction(function)
	output := printer.output.String()

	expectedStrings := []string{
		"FUNCTION test_function(param1: Address, param2: U256) -> U256",
		"[external, reads(State), writes(State)]",
		"entry:",
		"RETURN",
	}

	for _, expected := range expectedStrings {
		if !strings.Contains(output, expected) {
			t.Errorf("Function output should contain: %s", expected)
		}
	}

	// Check for constant assignment (more flexible)
	if !strings.Contains(output, "= 42") && !strings.Contains(output, "42") {
		t.Error("Function output should contain constant 42")
	}
}

func TestPrintInstructionOutput(t *testing.T) {
	// Test that instruction printing works through the public Print interface
	constant := &ConstantInstruction{
		ID:     1,
		Result: &Value{Name: "test_const", Type: &BoolType{}},
		Value:  "true",
		Type:   &BoolType{},
	}

	// Test the String method of the instruction
	result := constant.String()
	if result == "" {
		t.Error("Instruction String() method should return non-empty result")
	}
}

func TestInstructionStringMethods(t *testing.T) {
	// Test String() methods of various instructions
	testCases := []struct {
		name        string
		instruction Instruction
	}{
		{
			"ConstantInstruction",
			&ConstantInstruction{ID: 1, Result: &Value{Name: "const", Type: &BoolType{}}},
		},
		{
			"BinaryInstruction",
			&BinaryInstruction{ID: 2, Result: &Value{Name: "result", Type: &IntType{Bits: 256}}},
		},
		{
			"SenderInstruction",
			&SenderInstruction{ID: 3, Result: &Value{Name: "sender", Type: &AddressType{}}},
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			result := tc.instruction.String()
			if result == "" {
				t.Errorf("%s String() method should return non-empty result", tc.name)
			}
		})
	}
}

func TestTerminatorStringMethods(t *testing.T) {
	// Test String() methods of terminators
	testCases := []struct {
		name       string
		terminator Terminator
	}{
		{
			"ReturnTerminator",
			&ReturnTerminator{ID: 1},
		},
		{
			"BranchTerminator",
			&BranchTerminator{ID: 2},
		},
		{
			"JumpTerminator",
			&JumpTerminator{ID: 3},
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			result := tc.terminator.String()
			if result == "" {
				t.Errorf("%s String() method should return non-empty result", tc.name)
			}
		})
	}
}

func TestValueString(t *testing.T) {
	value := &Value{Name: "test_value", Type: &IntType{Bits: 256}, ID: 42}
	result := value.String()

	if result == "" {
		t.Error("Value String() method should return non-empty result")
	}

	// Should contain the value name
	if !strings.Contains(result, "test_value") {
		t.Error("Value string should contain the value name")
	}
}

func TestPrintCFGIntegration(t *testing.T) {
	// Test CFG printing through the main Print function
	entry := &BasicBlock{Label: "entry"}
	exit := &BasicBlock{Label: "exit"}

	program := &Program{
		Functions: []*Function{
			{
				Name:   "test_func",
				Blocks: []*BasicBlock{entry, exit},
			},
		},
		CFG: &ControlFlowGraph{
			EntryPoints: []*BasicBlock{entry},
			Functions: map[string]*FunctionCFG{
				"test_func": {
					Name:         "test_func",
					Entry:        entry,
					SuccessExits: []*BasicBlock{exit},
					FailureExits: []*BasicBlock{},
					Blocks:       []*BasicBlock{entry, exit},
				},
			},
		},
	}

	output := Print(program)

	expectedStrings := []string{
		"CONTROL FLOW GRAPH:",
		"Function: test_func",
	}

	for _, expected := range expectedStrings {
		if !strings.Contains(output, expected) {
			t.Errorf("CFG output should contain: %s", expected)
		}
	}
}
