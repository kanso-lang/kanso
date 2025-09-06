package semantic

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"kanso/internal/parser"
)

func TestBasicNameResolution(t *testing.T) {
	source := `#[contract]
module Test {
    struct Person {
        name: string,
        age: u32,
    }
    
    fun get_person(): Person {
        return Person { name: "test", age: 25 };
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Empty(t, semanticErrors, "Should have no semantic errors")
}

func TestDuplicateDeclarations(t *testing.T) {
	source := `#[contract]
module Test {
    fun test(): u32 {
        return 42;
    }
    
    fun test(): string {
        return "duplicate";
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 1, "Should have one semantic error")
	assert.Contains(t, semanticErrors[0].Message, "duplicate declaration")
}

func TestModuleRequiresAttribute(t *testing.T) {
	source := `module Test {
    fun test(): u32 {
        return 42;
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 1, "Should have one semantic error")
	assert.Contains(t, semanticErrors[0].Message, "module must have at least one attribute")
}

func TestContractRequiresModule(t *testing.T) {
	source := `// just a comment`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 1, "Should have one semantic error")
	assert.Contains(t, semanticErrors[0].Message, "contract must have at least one module")
}

func TestStructFunctionNameCollision(t *testing.T) {
	source := `#[contract]
module Test {
    struct test {
        value: u32,
    }
    
    fun test(): u32 {
        return 42;
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 1, "Should have one semantic error")
	assert.Contains(t, semanticErrors[0].Message, "duplicate declaration: test")
}
