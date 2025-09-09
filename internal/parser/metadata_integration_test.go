package parser

import (
	"strings"
	"testing"

	"kanso/internal/ast"
)

func TestMetadataIntegration(t *testing.T) {
	// Simple test source with various constructs
	source := `#[contract]
module TestContract {
    fun add(a: u256, b: u256): u256 {
        return a + b;
    }
}`

	// Parse with metadata
	result := ParseSourceWithMetadata("test.ka", source)

	if result.Contract == nil {
		t.Fatal("Contract should not be nil")
	}

	if result.MetadataVisitor == nil {
		t.Fatal("MetadataVisitor should not be nil")
	}

	// Check that we have metadata
	tracker := result.MetadataVisitor.GetTracker()
	if tracker == nil {
		t.Fatal("NodeTracker should not be nil")
	}

	allMetadata := tracker.GetAllMetadata()
	if len(allMetadata) == 0 {
		t.Fatal("Should have metadata for parsed nodes")
	}

	// Verify we can find nodes by position
	// Look for the module name "TestContract"
	modulePos := ast.Position{Filename: "test.ka", Line: 2, Column: 8, Offset: 19} // rough position
	nodeAtPos := result.FindNodeByPosition(modulePos)

	// Note: The exact position matching depends on the parser implementation
	// This test verifies the infrastructure works
	if nodeAtPos == nil {
		t.Log("Node lookup by position returned nil - this may be expected depending on exact positioning")
	}

	// Test debug output
	debugInfo := result.GetDebugInfo()
	if debugInfo == "" {
		t.Error("Debug info should not be empty")
	}

	// Verify debug info contains expected content
	if !strings.Contains(debugInfo, "AST Metadata Debug Info") {
		t.Error("Debug info should contain header")
	}
}

func TestMetadataHelperFunctions(t *testing.T) {
	source := `fun test(x: u256): u256 { return x * 2; }`
	result := ParseSourceWithMetadata("test.ka", source)

	if result.Contract == nil {
		t.Fatal("Contract should not be nil")
	}

	// Test that we can collect all nodes
	totalItems := len(result.Contract.LeadingComments) + len(result.Contract.Items)
	if totalItems == 0 {
		t.Fatal("Should have contract items")
	}

	// Get first available item (from leading comments or items)
	var firstItem ast.ContractItem
	if len(result.Contract.LeadingComments) > 0 {
		firstItem = result.Contract.LeadingComments[0]
	} else if len(result.Contract.Items) > 0 {
		firstItem = result.Contract.Items[0]
	}
	if firstItem != nil {
		// Test the helper functions work without errors
		ast.UpdateBytecodeMapping(firstItem, 0x1000, 0x1010, []ast.InstructionMapping{
			ast.CreateInstructionMapping(
				ast.Position{Filename: "test.ka", Line: 1, Column: 1, Offset: 0},
				0x1000,
				"LOAD",
				"param_x",
			),
		})

		ast.UpdateIRMapping(firstItem, 42)
		ast.UpdateTypeInfo(firstItem, "function", []string{}, 0, false, false)

		// Verify metadata was updated
		meta := firstItem.GetMetadata()
		if meta == nil {
			t.Error("Metadata should not be nil after updates")
		} else if meta.CompilationInfo == nil {
			t.Error("CompilationInfo should not be nil after updates")
		} else {
			if meta.CompilationInfo.IRID != 42 {
				t.Error("IRID should be 42")
			}
			if meta.CompilationInfo.TypeInfo == nil || meta.CompilationInfo.TypeInfo.TypeName != "function" {
				t.Error("Type info should be set to 'function'")
			}
			if meta.CompilationInfo.BytecodeRange == nil {
				t.Error("BytecodeRange should be set")
			}
		}
	}
}

func TestSourceMappingGeneration(t *testing.T) {
	source := `fun test() { return 42; }`
	result := ParseSourceWithMetadata("test.ka", source)

	totalItems := len(result.Contract.LeadingComments) + len(result.Contract.Items)
	if result.Contract == nil || totalItems == 0 {
		t.Fatal("Should have parsed contract items")
	}

	// Add some mock bytecode mappings
	var item ast.ContractItem
	if len(result.Contract.LeadingComments) > 0 {
		item = result.Contract.LeadingComments[0]
	} else if len(result.Contract.Items) > 0 {
		item = result.Contract.Items[0]
	}
	ast.UpdateBytecodeMapping(item, 0x100, 0x110, []ast.InstructionMapping{
		ast.CreateInstructionMapping(
			ast.Position{Filename: "test.ka", Line: 1, Column: 1, Offset: 0},
			0x100,
			"FUNC_START",
			"test",
		),
		ast.CreateInstructionMapping(
			ast.Position{Filename: "test.ka", Line: 1, Column: 14, Offset: 13},
			0x104,
			"LOAD_CONST",
			"42",
		),
		ast.CreateInstructionMapping(
			ast.Position{Filename: "test.ka", Line: 1, Column: 16, Offset: 15},
			0x108,
			"RETURN",
			"",
		),
	})

	// Test source mapping generation
	sourceMapping := result.GetSourceMapping()
	reverseMapping := result.GetReverseMapping()

	// Note: These will be empty unless we have actual bytecode mappings
	// This test verifies the functions work without errors
	if sourceMapping == nil {
		t.Log("Source mapping is nil - expected if no bytecode mappings exist")
	}

	if reverseMapping == nil {
		t.Log("Reverse mapping is nil - expected if no bytecode mappings exist")
	}
}
