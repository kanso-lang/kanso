package ir

import (
	"testing"
)

func TestNewOptimizationPipeline(t *testing.T) {
	pipeline := NewOptimizationPipeline()

	if pipeline == nil {
		t.Fatal("NewOptimizationPipeline should not return nil")
	}

	if len(pipeline.passes) == 0 {
		t.Error("OptimizationPipeline should have passes")
	}

	// Check that basic optimization passes are included
	if len(pipeline.passes) == 0 {
		t.Error("OptimizationPipeline should have optimization passes")
	}
}

func TestOptimizationPipelineRun(t *testing.T) {
	// Create a simple program to test optimization
	program := &Program{
		Functions: []*Function{
			{
				Name: "test_func",
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
						Terminator: &ReturnTerminator{},
					},
				},
			},
		},
		Constants: []*Constant{},
	}

	pipeline := NewOptimizationPipeline()
	pipeline.Run(program)

	// The optimization should run without errors
	// Specific behavior depends on the optimization passes
	if len(program.Functions) == 0 {
		t.Error("Program should still have functions after optimization")
	}
}

func TestSimpleOptimization(t *testing.T) {
	// Create test setup with basic instructions
	resultVal := &Value{Name: "result", Type: &IntType{Bits: 256}}

	block := &BasicBlock{
		Label: "test_block",
		Instructions: []Instruction{
			&ConstantInstruction{
				ID:     1,
				Result: resultVal,
				Value:  "42",
				Type:   &IntType{Bits: 256},
			},
		},
		Terminator: &ReturnTerminator{Value: resultVal},
	}

	function := &Function{
		Name:   "test_func",
		Blocks: []*BasicBlock{block},
	}

	program := &Program{
		Functions: []*Function{function},
	}

	// Test basic optimization pipeline
	pipeline := NewOptimizationPipeline()
	pipeline.Run(program)

	// Test that the optimization ran without errors
	if len(block.Instructions) == 0 {
		t.Error("Block should still have instructions after optimization")
	}
}

func TestOptimizationBasics(t *testing.T) {
	// Test that basic optimization classes can be created
	constantFolding := &ConstantFolding{}
	if constantFolding.Name() == "" {
		t.Error("ConstantFolding should have a non-empty name")
	}

	dce := &DeadCodeElimination{}
	if dce.Name() == "" {
		t.Error("DeadCodeElimination should have a non-empty name")
	}
}

func TestOptimizationWithEmptyProgram(t *testing.T) {
	program := &Program{
		Functions: []*Function{},
		Constants: []*Constant{},
	}

	pipeline := NewOptimizationPipeline()
	pipeline.Run(program)

	// Should not crash with empty program
	if len(program.Functions) != 0 {
		t.Error("Empty program should remain empty")
	}
}

func TestOptimizationWithMultipleFunctions(t *testing.T) {
	program := &Program{
		Functions: []*Function{
			{
				Name: "func1",
				Blocks: []*BasicBlock{
					{
						Label:        "entry1",
						Instructions: []Instruction{},
						Terminator:   &ReturnTerminator{},
					},
				},
			},
			{
				Name: "func2",
				Blocks: []*BasicBlock{
					{
						Label:        "entry2",
						Instructions: []Instruction{},
						Terminator:   &ReturnTerminator{},
					},
				},
			},
		},
		Constants: []*Constant{},
	}

	pipeline := NewOptimizationPipeline()
	pipeline.Run(program)

	// Should handle multiple functions
	if len(program.Functions) != 2 {
		t.Errorf("Program should have 2 functions after optimization, got %d", len(program.Functions))
	}
}

func TestOptimizationWithMultipleBlocks(t *testing.T) {
	block1 := &BasicBlock{
		Label:        "block1",
		Instructions: []Instruction{},
		Terminator:   &JumpTerminator{Target: nil}, // Would point to block2 in real scenario
	}

	block2 := &BasicBlock{
		Label:        "block2",
		Instructions: []Instruction{},
		Terminator:   &ReturnTerminator{},
	}

	function := &Function{
		Name:   "multi_block_func",
		Blocks: []*BasicBlock{block1, block2},
	}

	program := &Program{
		Functions: []*Function{function},
		Constants: []*Constant{},
	}

	pipeline := NewOptimizationPipeline()
	pipeline.Run(program)

	// Should handle multiple blocks per function
	// After optimization, some blocks might be eliminated
	blockCount := len(program.Functions[0].Blocks)
	if blockCount == 0 {
		t.Error("Function should have at least one block after optimization")
	} else if blockCount > 2 {
		t.Errorf("Function should have at most 2 blocks after optimization, got %d", blockCount)
	}
	// Accept 1 or 2 blocks as valid after optimization
}
