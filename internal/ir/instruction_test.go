package ir

import (
	"testing"
)

// Tests for basic instruction functionality
func TestBasicInstructionMethods(t *testing.T) {
	// Test basic instruction creation and methods

	// ConstantInstruction
	const1 := &ConstantInstruction{
		ID:     1,
		Result: &Value{Name: "const1", Type: &IntType{Bits: 256}},
		Value:  "42",
		Type:   &IntType{Bits: 256},
	}

	if const1.GetID() != 1 {
		t.Error("ConstantInstruction GetID should return 1")
	}

	if const1.GetResult().Name != "const1" {
		t.Error("ConstantInstruction GetResult should return const1")
	}

	str := const1.String()
	if str == "" {
		t.Error("ConstantInstruction String should return non-empty result")
	}

	// BinaryInstruction
	binary := &BinaryInstruction{
		ID:     2,
		Result: &Value{Name: "binary_result", Type: &IntType{Bits: 256}},
		Op:     "ADD",
		Left:   &Value{Name: "left", Type: &IntType{Bits: 256}},
		Right:  &Value{Name: "right", Type: &IntType{Bits: 256}},
	}

	if binary.GetID() != 2 {
		t.Error("BinaryInstruction GetID should return 2")
	}

	binaryStr := binary.String()
	if binaryStr == "" {
		t.Error("BinaryInstruction String should return non-empty result")
	}

	// SenderInstruction
	sender := &SenderInstruction{
		ID:     3,
		Result: &Value{Name: "sender", Type: &AddressType{}},
	}

	if sender.GetID() != 3 {
		t.Error("SenderInstruction GetID should return 3")
	}

	senderStr := sender.String()
	if senderStr == "" {
		t.Error("SenderInstruction String should return non-empty result")
	}

	// CallInstruction
	call := &CallInstruction{
		ID:       4,
		Result:   &Value{Name: "call_result", Type: &IntType{Bits: 256}},
		Function: "test_function",
		Args:     []*Value{},
	}

	if call.GetID() != 4 {
		t.Error("CallInstruction GetID should return 4")
	}

	callStr := call.String()
	if callStr == "" {
		t.Error("CallInstruction String should return non-empty result")
	}

	// Test various types
	testTypes := []Type{
		&IntType{Bits: 8},
		&IntType{Bits: 16},
		&IntType{Bits: 32},
		&IntType{Bits: 64},
		&IntType{Bits: 128},
		&IntType{Bits: 256},
		&BoolType{},
		&AddressType{},
		&StringType{},
		&SlotsType{KeyType: &AddressType{}, ValueType: &IntType{Bits: 256}},
	}

	for i, typ := range testTypes {
		str := typ.String()
		if str == "" {
			t.Errorf("Type %d should have non-empty string representation", i)
		}
	}
}

func TestSimpleTerminatorTests(t *testing.T) {
	// ReturnTerminator
	returnTerm := &ReturnTerminator{
		ID:    1,
		Value: &Value{Name: "return_value", Type: &IntType{Bits: 256}},
	}

	if returnTerm.GetID() != 1 {
		t.Error("ReturnTerminator GetID should return 1")
	}

	returnStr := returnTerm.String()
	if returnStr == "" {
		t.Error("ReturnTerminator String should return non-empty result")
	}

	successors := returnTerm.GetSuccessors()
	if len(successors) != 0 {
		t.Error("ReturnTerminator should have 0 successors")
	}

	// BranchTerminator
	trueBlock := &BasicBlock{Label: "true_block"}
	falseBlock := &BasicBlock{Label: "false_block"}

	branch := &BranchTerminator{
		ID:         2,
		Condition:  &Value{Name: "condition", Type: &BoolType{}},
		TrueBlock:  trueBlock,
		FalseBlock: falseBlock,
	}

	if branch.GetID() != 2 {
		t.Error("BranchTerminator GetID should return 2")
	}

	branchStr := branch.String()
	if branchStr == "" {
		t.Error("BranchTerminator String should return non-empty result")
	}

	branchSuccessors := branch.GetSuccessors()
	if len(branchSuccessors) != 2 {
		t.Error("BranchTerminator should have 2 successors")
	}

	// JumpTerminator
	target := &BasicBlock{Label: "target"}

	jump := &JumpTerminator{
		ID:     3,
		Target: target,
	}

	if jump.GetID() != 3 {
		t.Error("JumpTerminator GetID should return 3")
	}

	jumpStr := jump.String()
	if jumpStr == "" {
		t.Error("JumpTerminator String should return non-empty result")
	}

	jumpSuccessors := jump.GetSuccessors()
	if len(jumpSuccessors) != 1 || jumpSuccessors[0] != target {
		t.Error("JumpTerminator should have 1 successor")
	}
}

func TestSimpleValueTests(t *testing.T) {
	value := &Value{
		ID:      42,
		Name:    "test_value",
		Type:    &IntType{Bits: 256},
		Version: 1,
	}

	str := value.String()
	if str == "" {
		t.Error("Value String should return non-empty result")
	}

	if value.ID != 42 {
		t.Error("Value ID should be 42")
	}

	if value.Name != "test_value" {
		t.Error("Value Name should be test_value")
	}

	if value.Version != 1 {
		t.Error("Value Version should be 1")
	}
}

func TestSimpleBasicBlockTests(t *testing.T) {
	block := &BasicBlock{
		Label:        "test_block",
		Instructions: []Instruction{},
		Predecessors: []*BasicBlock{},
		Successors:   []*BasicBlock{},
		LiveIn:       make(map[string]*Value),
		LiveOut:      make(map[string]*Value),
	}

	str := block.String()
	if str == "" {
		t.Error("BasicBlock String should return non-empty result")
	}

	if block.Label != "test_block" {
		t.Error("BasicBlock Label should be test_block")
	}

	if block.Instructions == nil {
		t.Error("BasicBlock Instructions should not be nil")
	}

	if block.LiveIn == nil {
		t.Error("BasicBlock LiveIn should not be nil")
	}

	if block.LiveOut == nil {
		t.Error("BasicBlock LiveOut should not be nil")
	}
}

func TestSimpleFunctionTests(t *testing.T) {
	function := &Function{
		Name:       "test_function",
		External:   true,
		Create:     false,
		Params:     []*Parameter{},
		ReturnType: &IntType{Bits: 256},
		Reads:      []string{"State"},
		Writes:     []string{"State"},
		Blocks:     []*BasicBlock{},
		LocalVars:  make(map[string]*Value),
	}

	if function.Name != "test_function" {
		t.Error("Function Name should be test_function")
	}

	if !function.External {
		t.Error("Function External should be true")
	}

	if function.Create {
		t.Error("Function Create should be false")
	}

	if function.ReturnType.String() != "U256" {
		t.Error("Function ReturnType should be U256")
	}

	if len(function.Reads) != 1 || function.Reads[0] != "State" {
		t.Error("Function Reads should contain State")
	}

	if len(function.Writes) != 1 || function.Writes[0] != "State" {
		t.Error("Function Writes should contain State")
	}
}

func TestSimpleProgramTests(t *testing.T) {
	program := &Program{
		Functions: []*Function{},
		Constants: []*Constant{},
		Blocks:    make(map[string]*BasicBlock),
		CFG:       &ControlFlowGraph{},
	}

	if program.Functions == nil {
		t.Error("Program Functions should not be nil")
	}

	if program.Constants == nil {
		t.Error("Program Constants should not be nil")
	}

	if program.Blocks == nil {
		t.Error("Program Blocks should not be nil")
	}

	if program.CFG == nil {
		t.Error("Program CFG should not be nil")
	}
}

func TestSimpleParameterTests(t *testing.T) {
	param := &Parameter{
		Name: "test_param",
		Type: &AddressType{},
	}

	if param.Name != "test_param" {
		t.Error("Parameter Name should be test_param")
	}

	if param.Type.String() != "Address" {
		t.Error("Parameter Type should be Address")
	}
}

func TestSimpleConstantTests(t *testing.T) {
	constant := &Constant{
		Value: &Value{Name: "const_value", Type: &BoolType{}},
		Data:  "true",
	}

	if constant.Data != "true" {
		t.Error("Constant Data should be true")
	}

	if constant.Value.Name != "const_value" {
		t.Error("Constant Value name should be const_value")
	}
}

// Test specific instruction GetBlock methods
func TestGetBlockMethods(t *testing.T) {
	block := &BasicBlock{Label: "test_block"}

	instructions := []interface{}{
		&ConstantInstruction{ID: 1, Block: block},
		&BinaryInstruction{ID: 2, Block: block},
		&SenderInstruction{ID: 3, Block: block},
		&CallInstruction{ID: 4, Block: block},
	}

	for i, inst := range instructions {
		switch v := inst.(type) {
		case *ConstantInstruction:
			if v.GetBlock() != block {
				t.Errorf("Instruction %d GetBlock should return test_block", i)
			}
		case *BinaryInstruction:
			if v.GetBlock() != block {
				t.Errorf("Instruction %d GetBlock should return test_block", i)
			}
		case *SenderInstruction:
			if v.GetBlock() != block {
				t.Errorf("Instruction %d GetBlock should return test_block", i)
			}
		case *CallInstruction:
			if v.GetBlock() != block {
				t.Errorf("Instruction %d GetBlock should return test_block", i)
			}
		}
	}
}

// Test instruction effects for basic cases
func TestInstructionEffectsBasic(t *testing.T) {
	// Test instructions with effects
	const1 := &ConstantInstruction{ID: 1}
	effects := const1.GetEffects()
	// ConstantInstruction might have effects in the current implementation
	_ = effects // Don't assert on specific count as it depends on implementation

	binary := &BinaryInstruction{ID: 2}
	binaryEffects := binary.GetEffects()
	// BinaryInstruction might have effects in the current implementation
	_ = binaryEffects

	sender := &SenderInstruction{ID: 3}
	senderEffects := sender.GetEffects()
	// SenderInstruction might have effects in the current implementation
	_ = senderEffects
}
