package ir

import (
	"testing"
)

func TestIntTypeString(t *testing.T) {
	testCases := []struct {
		bits     int
		expected string
	}{
		{8, "U8"},
		{16, "U16"},
		{32, "U32"},
		{64, "U64"},
		{128, "U128"},
		{256, "U256"},
		{123, "U123"},
	}

	for _, tc := range testCases {
		intType := &IntType{Bits: tc.bits}
		result := intType.String()
		if result != tc.expected {
			t.Errorf("IntType{Bits: %d}.String() = %s, expected %s", tc.bits, result, tc.expected)
		}
	}
}

func TestBoolTypeString(t *testing.T) {
	boolType := &BoolType{}
	result := boolType.String()
	expected := "Bool"
	if result != expected {
		t.Errorf("BoolType.String() = %s, expected %s", result, expected)
	}
}

func TestAddressTypeString(t *testing.T) {
	addressType := &AddressType{}
	result := addressType.String()
	expected := "Address"
	if result != expected {
		t.Errorf("AddressType.String() = %s, expected %s", result, expected)
	}
}

func TestStringTypeString(t *testing.T) {
	stringType := &StringType{}
	result := stringType.String()
	expected := "String"
	if result != expected {
		t.Errorf("StringType.String() = %s, expected %s", result, expected)
	}
}

func TestSlotsTypeString(t *testing.T) {
	keyType := &AddressType{}
	valueType := &IntType{Bits: 256}
	slotsType := &SlotsType{KeyType: keyType, ValueType: valueType}
	result := slotsType.String()
	if result == "" {
		t.Error("SlotsType.String() should return non-empty string")
	}
}

func TestMemoryRegion(t *testing.T) {
	baseVal := &Value{Name: "base", Type: &IntType{Bits: 256}}
	sizeVal := &Value{Name: "size", Type: &IntType{Bits: 256}}

	region := &MemoryRegion{
		ID:   1,
		Name: "test_region",
		Base: baseVal,
		Size: sizeVal,
		Kind: MemoryRegionABIData,
	}

	if region.Name != "test_region" {
		t.Errorf("MemoryRegion.Name = %s, expected test_region", region.Name)
	}

	if region.ID != 1 {
		t.Errorf("MemoryRegion.ID = %d, expected 1", region.ID)
	}

	if region.Kind != MemoryRegionABIData {
		t.Errorf("MemoryRegion.Kind = %s, expected %s", region.Kind, MemoryRegionABIData)
	}
}

func TestMemoryEffect(t *testing.T) {
	region := &MemoryRegion{Name: "test_region"}
	offsetVal := &Value{Name: "offset", Type: &IntType{Bits: 256}}
	sizeVal := &Value{Name: "size", Type: &IntType{Bits: 256}}

	effect := &MemoryEffect{
		Region: region,
		Type:   MemoryEffectRead,
		Offset: offsetVal,
		Size:   sizeVal,
	}

	if effect.Type != MemoryEffectRead {
		t.Errorf("MemoryEffect.Type = %v, expected MemoryEffectRead", effect.Type)
	}

	if effect.Region != region {
		t.Error("MemoryEffect.Region should match the assigned region")
	}

	if effect.Offset != offsetVal {
		t.Error("MemoryEffect.Offset should match the assigned value")
	}

	if effect.Size != sizeVal {
		t.Error("MemoryEffect.Size should match the assigned value")
	}
}

func TestMemoryEffectOp(t *testing.T) {
	region := &MemoryRegion{Name: "test_region"}
	effectOp := &MemoryEffectOp{
		Type:   MemoryEffectWrite,
		Region: region,
	}

	if effectOp.Type != MemoryEffectWrite {
		t.Errorf("MemoryEffectOp.Type = %v, expected MemoryEffectWrite", effectOp.Type)
	}

	if effectOp.Region != region {
		t.Error("MemoryEffectOp.Region should match the assigned region")
	}
}

func TestStorageEffect(t *testing.T) {
	storageEffect := &StorageEffect{
		Type: "write",
		Slot: 5,
	}

	if storageEffect.Type != "write" {
		t.Errorf("StorageEffect.Type = %s, expected write", storageEffect.Type)
	}

	if storageEffect.Slot != 5 {
		t.Errorf("StorageEffect.Slot = %d, expected 5", storageEffect.Slot)
	}
}

func TestPureEffect(t *testing.T) {
	pureEffect := &PureEffect{}
	// PureEffect is an empty struct, just test that it can be created
	if pureEffect == nil {
		t.Error("PureEffect should not be nil")
	}
}

func TestValue(t *testing.T) {
	valueType := &IntType{Bits: 256}
	value := &Value{
		ID:      1,
		Name:    "test_value",
		Type:    valueType,
		Version: 2,
	}

	if value.ID != 1 {
		t.Errorf("Value.ID = %d, expected 1", value.ID)
	}

	if value.Name != "test_value" {
		t.Errorf("Value.Name = %s, expected test_value", value.Name)
	}

	if value.Type != valueType {
		t.Error("Value.Type should match assigned type")
	}

	if value.Version != 2 {
		t.Errorf("Value.Version = %d, expected 2", value.Version)
	}
}

func TestBasicBlock(t *testing.T) {
	block := &BasicBlock{
		Label:        "test_block",
		Instructions: []Instruction{},
		Predecessors: []*BasicBlock{},
		Successors:   []*BasicBlock{},
		LiveIn:       make(map[string]*Value),
		LiveOut:      make(map[string]*Value),
	}

	if block.Label != "test_block" {
		t.Errorf("BasicBlock.Label = %s, expected test_block", block.Label)
	}

	if block.Instructions == nil {
		t.Error("BasicBlock.Instructions should not be nil")
	}

	if block.LiveIn == nil {
		t.Error("BasicBlock.LiveIn should not be nil")
	}

	if block.LiveOut == nil {
		t.Error("BasicBlock.LiveOut should not be nil")
	}
}

func TestFunction(t *testing.T) {
	function := &Function{
		Name:       "test_function",
		External:   true,
		Create:     false,
		Params:     []*Parameter{},
		ReturnType: &IntType{Bits: 256},
		Reads:      []string{"State"},
		Writes:     []string{},
		Blocks:     []*BasicBlock{},
		LocalVars:  make(map[string]*Value),
	}

	if function.Name != "test_function" {
		t.Errorf("Function.Name = %s, expected test_function", function.Name)
	}

	if !function.External {
		t.Error("Function.External should be true")
	}

	if function.Create {
		t.Error("Function.Create should be false")
	}

	if function.ReturnType.String() != "U256" {
		t.Errorf("Function.ReturnType = %s, expected U256", function.ReturnType.String())
	}

	if len(function.Reads) != 1 || function.Reads[0] != "State" {
		t.Errorf("Function.Reads = %v, expected [State]", function.Reads)
	}

	if function.LocalVars == nil {
		t.Error("Function.LocalVars should not be nil")
	}
}

func TestParameter(t *testing.T) {
	paramType := &AddressType{}
	param := &Parameter{
		Name: "test_param",
		Type: paramType,
	}

	if param.Name != "test_param" {
		t.Errorf("Parameter.Name = %s, expected test_param", param.Name)
	}

	if param.Type != paramType {
		t.Error("Parameter.Type should match assigned type")
	}
}

func TestProgram(t *testing.T) {
	program := &Program{
		Functions: []*Function{},
		Constants: []*Constant{},
	}

	if program.Functions == nil {
		t.Error("Program.Functions should not be nil")
	}

	if program.Constants == nil {
		t.Error("Program.Constants should not be nil")
	}
}

func TestConstant(t *testing.T) {
	valueType := &BoolType{}
	value := &Value{Name: "test_value", Type: valueType}
	constant := &Constant{
		Value: value,
		Data:  "true",
	}

	if constant.Value != value {
		t.Error("Constant.Value should match assigned value")
	}

	if constant.Data != "true" {
		t.Errorf("Constant.Data = %s, expected true", constant.Data)
	}
}

func TestControlFlowGraph(t *testing.T) {
	cfg := &ControlFlowGraph{
		EntryPoints:  []*BasicBlock{},
		SuccessExits: []*BasicBlock{},
		FailureExits: []*BasicBlock{},
		Blocks:       []*BasicBlock{},
		Dominance:    make(map[*BasicBlock][]*BasicBlock),
		Loops:        []*Loop{},
		Functions:    make(map[string]*FunctionCFG),
	}

	if cfg.EntryPoints == nil {
		t.Error("ControlFlowGraph.EntryPoints should not be nil")
	}

	if cfg.Dominance == nil {
		t.Error("ControlFlowGraph.Dominance should not be nil")
	}

	if cfg.Functions == nil {
		t.Error("ControlFlowGraph.Functions should not be nil")
	}
}

func TestFunctionCFG(t *testing.T) {
	entry := &BasicBlock{Label: "entry"}
	exit := &BasicBlock{Label: "exit"}

	fnCFG := &FunctionCFG{
		Name:         "test_function",
		Entry:        entry,
		SuccessExits: []*BasicBlock{exit},
		FailureExits: []*BasicBlock{},
		Blocks:       []*BasicBlock{entry, exit},
	}

	if fnCFG.Name != "test_function" {
		t.Errorf("FunctionCFG.Name = %s, expected test_function", fnCFG.Name)
	}

	if fnCFG.Entry != entry {
		t.Error("FunctionCFG.Entry should match assigned entry block")
	}

	if len(fnCFG.SuccessExits) != 1 || fnCFG.SuccessExits[0] != exit {
		t.Error("FunctionCFG.SuccessExits should contain the exit block")
	}

	if len(fnCFG.Blocks) != 2 {
		t.Errorf("FunctionCFG.Blocks should have 2 blocks, got %d", len(fnCFG.Blocks))
	}
}

func TestLoop(t *testing.T) {
	header := &BasicBlock{Label: "loop_header"}
	body := &BasicBlock{Label: "loop_body"}
	exit := &BasicBlock{Label: "loop_exit"}

	loop := &Loop{
		Header:    header,
		Body:      []*BasicBlock{body},
		Exits:     []*BasicBlock{exit},
		Invariant: []*Value{},
	}

	if loop.Header != header {
		t.Error("Loop.Header should match assigned header block")
	}

	if len(loop.Body) != 1 || loop.Body[0] != body {
		t.Error("Loop.Body should contain the body block")
	}

	if len(loop.Exits) != 1 || loop.Exits[0] != exit {
		t.Error("Loop.Exits should contain the exit block")
	}

	if loop.Invariant == nil {
		t.Error("Loop.Invariant should not be nil")
	}
}
