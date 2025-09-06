package ast

import (
	"testing"
)

func TestNodeTracker(t *testing.T) {
	tracker := NewNodeTracker()

	// Test ID generation
	id1 := tracker.GenerateID()
	id2 := tracker.GenerateID()

	if id1 == id2 {
		t.Error("GenerateID should return unique IDs")
	}

	if id1 != 1 || id2 != 2 {
		t.Errorf("Expected IDs 1,2 but got %d,%d", id1, id2)
	}
}

func TestMetadata(t *testing.T) {
	// Test metadata creation and manipulation
	pos1 := Position{Filename: "test.ka", Line: 1, Column: 1, Offset: 0}
	pos2 := Position{Filename: "test.ka", Line: 1, Column: 10, Offset: 9}

	sourceRange := CreateSourceRange(pos1, pos2)

	if !sourceRange.Contains(Position{Filename: "test.ka", Line: 1, Column: 5, Offset: 4}) {
		t.Error("SourceRange should contain position within range")
	}

	if sourceRange.Contains(Position{Filename: "test.ka", Line: 2, Column: 1, Offset: 20}) {
		t.Error("SourceRange should not contain position outside range")
	}
}

func TestSourceRange(t *testing.T) {
	start := Position{Filename: "test.ka", Line: 1, Column: 1, Offset: 0}
	end := Position{Filename: "test.ka", Line: 1, Column: 10, Offset: 9}

	sr := CreateSourceRange(start, end)

	// Test string representation
	expected := "test.ka:1:1-10"
	if sr.String() != expected {
		t.Errorf("Expected %s but got %s", expected, sr.String())
	}

	// Test multiline range
	endMulti := Position{Filename: "test.ka", Line: 2, Column: 5, Offset: 15}
	srMulti := CreateSourceRange(start, endMulti)
	expectedMulti := "test.ka:1:1-2:5"
	if srMulti.String() != expectedMulti {
		t.Errorf("Expected %s but got %s", expectedMulti, srMulti.String())
	}
}

func TestBytecodeRange(t *testing.T) {
	// Test bytecode range and instruction mapping
	pos1 := Position{Filename: "test.ka", Line: 1, Column: 1, Offset: 0}

	instr1 := CreateInstructionMapping(pos1, 0x100, "LOAD", "R1, #42")
	instr2 := CreateInstructionMapping(pos1, 0x104, "STORE", "R1, @var")

	bcRange := &BytecodeRange{
		StartAddress: 0x100,
		EndAddress:   0x108,
		Instructions: []InstructionMapping{instr1, instr2},
	}

	if bcRange.StartAddress != 0x100 || bcRange.EndAddress != 0x108 {
		t.Error("BytecodeRange addresses not set correctly")
	}

	if len(bcRange.Instructions) != 2 {
		t.Error("BytecodeRange should contain 2 instructions")
	}

	if bcRange.Instructions[0].Instruction != "LOAD" {
		t.Error("First instruction should be LOAD")
	}
}

func TestMetadataVisitor(t *testing.T) {
	sourceText := "fun test() { return 42; }"
	visitor := NewMetadataVisitor(sourceText)

	// Test source text extraction
	start := Position{Filename: "test.ka", Line: 1, Column: 1, Offset: 0}
	end := Position{Filename: "test.ka", Line: 1, Column: 3, Offset: 2}

	extracted := visitor.extractSourceText(start, end)
	expected := "fu"

	if extracted != expected {
		t.Errorf("Expected '%s' but got '%s'", expected, extracted)
	}
}

func TestCompilationMetadata(t *testing.T) {
	// Test that compilation metadata can be added and retrieved
	meta := &Metadata{
		NodeID: 1,
		Source: CreateSourceRange(
			Position{Filename: "test.ka", Line: 1, Column: 1, Offset: 0},
			Position{Filename: "test.ka", Line: 1, Column: 10, Offset: 9},
		),
		SourceText: "test_code",
		ParentID:   0,
	}

	// Add compilation info
	meta.CompilationInfo = &CompilationMetadata{
		IRID: 42,
		BytecodeRange: &BytecodeRange{
			StartAddress: 0x1000,
			EndAddress:   0x1010,
		},
		TypeInfo: &TypeMetadata{
			TypeName:    "u256",
			SizeBytes:   32,
			IsReference: false,
			IsMutable:   false,
		},
	}

	if meta.CompilationInfo.IRID != 42 {
		t.Error("IRID not set correctly")
	}

	if meta.CompilationInfo.BytecodeRange.StartAddress != 0x1000 {
		t.Error("Bytecode start address not set correctly")
	}

	if meta.CompilationInfo.TypeInfo.TypeName != "u256" {
		t.Error("Type name not set correctly")
	}
}

func TestOptimizationTracking(t *testing.T) {
	// Test optimization tracking
	optInfo := &OptimizationInfo{
		OptimizedOut:       false,
		OptimizationPasses: []string{},
		InlinedFrom:        nil,
		ConstantFolded:     false,
		OriginalValue:      "",
	}

	// Mark as optimized
	optInfo.OptimizedOut = true
	optInfo.OptimizationPasses = append(optInfo.OptimizationPasses, "constant_folding")
	optInfo.ConstantFolded = true
	optInfo.OriginalValue = "2 + 3"

	if !optInfo.OptimizedOut {
		t.Error("Should be marked as optimized out")
	}

	if len(optInfo.OptimizationPasses) != 1 || optInfo.OptimizationPasses[0] != "constant_folding" {
		t.Error("Optimization pass not recorded correctly")
	}

	if !optInfo.ConstantFolded {
		t.Error("Should be marked as constant folded")
	}

	if optInfo.OriginalValue != "2 + 3" {
		t.Error("Original value not recorded correctly")
	}
}
