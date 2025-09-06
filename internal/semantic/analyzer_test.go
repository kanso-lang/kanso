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

func TestInvalidModuleAttribute(t *testing.T) {
	source := `#[invalid]
module Test {
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
	assert.Contains(t, semanticErrors[0].Message, "invalid module attribute: invalid")
}

func TestInvalidStructAttribute(t *testing.T) {
	source := `#[contract]
module Test {
    #[invalid]
    struct Test {
        value: u32,
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 1, "Should have one semantic error")
	assert.Contains(t, semanticErrors[0].Message, "invalid struct attribute: invalid")
}

func TestInvalidFunctionAttribute(t *testing.T) {
	source := `#[contract]
module Test {
    #[invalid]
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
	assert.Contains(t, semanticErrors[0].Message, "invalid function attribute: invalid")
}

func TestMultipleCreateFunctions(t *testing.T) {
	source := `#[contract]
module Test {
    #[storage]
    struct State {
        value: u32,
    }
    
    #[create]
    fun create1() writes State {
        // constructor logic
    }
    
    #[create]
    fun create2() writes State {
        // constructor logic
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 1, "Should have one semantic error")
	assert.Contains(t, semanticErrors[0].Message, "multiple functions with #[create] attribute found")
}

func TestConstructorWithReturnType(t *testing.T) {
	source := `#[contract]
module Test {
    #[storage]
    struct State {
        value: u32,
    }
    
    #[create]
    fun create(): u32 writes State {
        return 42;
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 1, "Should have one semantic error")
	assert.Contains(t, semanticErrors[0].Message, "constructor functions cannot have a return type")
}

func TestConstructorWithoutWrites(t *testing.T) {
	source := `#[contract]
module Test {
    #[storage]
    struct State {
        value: u32,
    }
    
    #[create]
    fun create() {
        // no writes clause
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 1, "Should have one semantic error")
	assert.Contains(t, semanticErrors[0].Message, "constructor functions must have a writes clause")
}

func TestConstructorWithoutStorageWrite(t *testing.T) {
	source := `#[contract]
module Test {
    #[storage]
    struct State {
        value: u32,
    }
    
    #[create]
    fun create() writes SomethingElse {
        // writes to non-storage struct
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 2, "Should have two semantic errors")
	// Both general writes validation and constructor validation should trigger
	foundGeneralError := false
	foundConstructorError := false
	for _, err := range semanticErrors {
		if err.Message == "writes clause references non-storage struct: SomethingElse" {
			foundGeneralError = true
		}
		if err.Message == "constructor functions must write to a storage struct" {
			foundConstructorError = true
		}
	}
	assert.True(t, foundGeneralError, "Should have general writes validation error")
	assert.True(t, foundConstructorError, "Should have constructor validation error")
}

func TestConstructorWritesToEventStruct(t *testing.T) {
	source := `#[contract]
module Test {
    #[event]
    struct Transfer {
        from: address,
        to: address,
    }
    
    #[create]
    fun create() writes Transfer {
        // writes to event struct, not storage
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 2, "Should have two semantic errors")
	// Both general writes validation and constructor validation should trigger
	foundGeneralError := false
	foundConstructorError := false
	for _, err := range semanticErrors {
		if err.Message == "writes clause references non-storage struct: Transfer" {
			foundGeneralError = true
		}
		if err.Message == "constructor functions must write to a storage struct" {
			foundConstructorError = true
		}
	}
	assert.True(t, foundGeneralError, "Should have general writes validation error")
	assert.True(t, foundConstructorError, "Should have constructor validation error")
}

func TestConstructorWritesToStructWithoutAttribute(t *testing.T) {
	source := `#[contract]
module Test {
    struct RegularStruct {
        value: u32,
    }
    
    #[create]
    fun create() writes RegularStruct {
        // writes to struct without attribute
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 2, "Should have two semantic errors")
	// One error for constructor validation, one for general writes validation
	assert.True(t, len(semanticErrors) >= 1, "Should have semantic errors")
}

func TestFunctionReadsNonStorageStruct(t *testing.T) {
	source := `#[contract]
module Test {
    #[storage]
    struct State {
        value: u32,
    }
    
    struct RegularStruct {
        data: u32,
    }
    
    fun test() reads RegularStruct {
        // reads from non-storage struct
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 1, "Should have one semantic error")
	assert.Contains(t, semanticErrors[0].Message, "reads clause references non-storage struct: RegularStruct")
}

func TestFunctionWritesNonStorageStruct(t *testing.T) {
	source := `#[contract]
module Test {
    #[storage]
    struct State {
        value: u32,
    }
    
    #[event]
    struct Transfer {
        from: address,
        to: address,
    }
    
    fun test() writes Transfer {
        // writes to event struct, not storage
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 1, "Should have one semantic error")
	assert.Contains(t, semanticErrors[0].Message, "writes clause references non-storage struct: Transfer")
}

func TestValidFunctionReadsWrites(t *testing.T) {
	source := `#[contract]
module Test {
    #[storage]
    struct State {
        value: u32,
    }
    
    #[storage]
    struct Config {
        setting: bool,
    }
    
    fun test() reads State writes Config {
        // valid reads and writes to storage structs
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Empty(t, semanticErrors, "Should have no semantic errors")
}

func TestConflictingReadsWritesClause(t *testing.T) {
	source := `#[contract]
module Test {
    #[storage]
    struct State {
        value: u32,
    }
    
    fun test() reads State writes State {
        // conflicting read and write to same struct
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 1, "Should have one semantic error")
	assert.Contains(t, semanticErrors[0].Message, "conflicting reads and writes clause for struct (write implies read): State")
}

func TestValidMixedReadsWrites(t *testing.T) {
	source := `#[contract]
module Test {
    #[storage]
    struct State1 {
        value: u32,
    }
    
    #[storage]
    struct State2 {
        config: bool,
    }
    
    fun test() reads State1 writes State2 {
        // valid: read from one struct, write to different struct
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Empty(t, semanticErrors, "Should have no semantic errors")
}
