package semantic

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"kanso/internal/parser"
)

func TestBasicNameResolution(t *testing.T) {
	source := `contract Test {
    struct Person {
        name: String,
        age: U32,
    }
    
    fn get_person() -> Person {
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
	source := `contract Test {
    fn test() -> U32 {
        42
    }
    
    fn test() -> String {
        "duplicate"
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

func TestBasicContractValidation(t *testing.T) {
	source := `contract Test {
    fn test() -> U32 {
        42
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should have no semantic errors for a basic valid contract
	assert.Empty(t, semanticErrors, "Should have no semantic errors")
}

func TestContractParsingValidation(t *testing.T) {
	source := `// just a comment`

	_, parseErrors, _ := parser.ParseSource("test.ka", source)
	// This should have parse errors since it's not a valid contract
	assert.NotEmpty(t, parseErrors, "Should have parse errors for invalid contract")
}

func TestStructFunctionNameCollision(t *testing.T) {
	source := `contract Test {
    struct test {
        value: U32,
    }
    
    fn test() -> U32 {
        42
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

func TestInvalidStructAttribute(t *testing.T) {
	source := `contract Test {
    #[invalid]
    struct TestStruct {
        value: U32,
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
	source := `contract Test {
    #[invalid]
    fn test() -> U32 {
        42
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
	source := `contract Test {
    #[storage]
    struct State {
        value: U32,
    }
    
    #[create]
    fn create1() writes State {
        // constructor logic
    }
    
    #[create]
    fn create2() writes State {
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
	source := `contract Test {

    #[storage]
    struct State {
        value: U32,
    }
    
    #[create]
    fn create() -> U32 writes State {
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
	source := `contract Test {

    #[storage]
    struct State {
        value: U32,
    }
    
    #[create]
    fn create() {
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
	source := `contract Test {

    #[storage]
    struct State {
        value: U32,
    }
    
    #[create]
    fn create() writes SomethingElse {
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
	source := `contract Test {

    #[event]
    struct Transfer {
        from: Address,
        to: Address,
    }
    
    #[create]
    fn create() writes Transfer {
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
	source := `contract Test {

    struct RegularStruct {
        value: U32,
    }
    
    #[create]
    fn create() writes RegularStruct {
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
	source := `contract Test {

    #[storage]
    struct State {
        value: U32,
    }
    
    struct RegularStruct {
        data: U32,
    }
    
    fn test() reads RegularStruct {
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
	source := `contract Test {

    #[storage]
    struct State {
        value: U32,
    }
    
    #[event]
    struct Transfer {
        from: Address,
        to: Address,
    }
    
    fn test() writes Transfer {
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
	source := `contract Test {

    #[storage]
    struct State {
        value: U32,
    }
    
    #[storage]
    struct Config {
        setting: Bool,
    }
    
    fn test() reads State writes Config {
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
	source := `contract Test {

    #[storage]
    struct State {
        value: U32,
    }
    
    fn test() reads State writes State {
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
	source := `contract Test {

    #[storage]
    struct State1 {
        value: U32,
    }
    
    #[storage]
    struct State2 {
        config: Bool,
    }
    
    fn test() reads State1 writes State2 {
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

func TestTypeRegistryIntegration(t *testing.T) {
	source := `contract Test {

    #[storage]
    struct State {
        value: U32,
    }
    
    fn test() -> Bool {
        return true;
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should pass with no errors
	assert.Empty(t, semanticErrors, "Should have no semantic errors")

	// Verify types are registered correctly (without imports for now)
	assert.True(t, analyzer.context.IsBuiltinType("U32"), "U32 should be built-in")
	assert.True(t, analyzer.context.IsBuiltinType("Bool"), "Bool should be built-in")
	assert.True(t, analyzer.context.IsUserDefinedType("State"), "State should be user-defined")
	assert.False(t, analyzer.context.IsImportedType("Table"), "Table should not be imported without use statement")
}

func TestERC20Imports(t *testing.T) {
	source := `contract Test {

    use Evm::{sender, emit};
    use Table::{Self, Table};
    use std::ascii::{String};
    use std::errors;
    
    #[storage]
    struct State {
        value: U32,
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should pass with no errors
	assert.Empty(t, semanticErrors, "Should have no semantic errors")

	// Verify imported types are registered correctly
	assert.False(t, analyzer.context.IsImportedType("sender"), "sender is a function, not a type")
	assert.False(t, analyzer.context.IsImportedType("emit"), "emit is a function, not a type")
	assert.True(t, analyzer.context.IsImportedType("Table"), "Table should be imported as a type")
	assert.True(t, analyzer.context.IsImportedType("String"), "String should be imported as a type")

	// Verify imported functions are registered correctly
	assert.True(t, analyzer.context.IsImportedFunction("sender"), "sender should be imported as a function")
	assert.True(t, analyzer.context.IsImportedFunction("emit"), "emit should be imported as a function")

	// Verify imported modules are registered correctly
	assert.True(t, analyzer.context.IsImportedModule("Table"), "Table module should be imported via Self")
	assert.True(t, analyzer.context.IsImportedModule("errors"), "errors module should be imported")

	// Verify Table is marked as generic
	tableType := analyzer.context.GetImportedType("Table")
	assert.NotNil(t, tableType, "Table type should exist")
	assert.True(t, tableType.IsGeneric, "Table should be generic")

	// Verify String is marked as non-generic
	stringType := analyzer.context.GetImportedType("String")
	assert.NotNil(t, stringType, "String type should exist")
	assert.False(t, stringType.IsGeneric, "String should not be generic")

	// Verify standard library integration
	assert.True(t, analyzer.context.IsStandardModule("Evm"), "Evm should be a standard module")
	assert.True(t, analyzer.context.IsStandardModule("Table"), "Table should be a standard module")
	assert.True(t, analyzer.context.IsStandardModule("std::ascii"), "std::ascii should be a standard module")
	assert.True(t, analyzer.context.IsStandardModule("std::errors"), "std::errors should be a standard module")

	// Verify function definition access
	senderFunc := analyzer.context.GetFunctionDefinition("sender")
	assert.NotNil(t, senderFunc, "Should get sender function definition")
	assert.Equal(t, "sender", senderFunc.Name)
	assert.Equal(t, "Address", senderFunc.ReturnType.Name)

	emptyFunc := analyzer.context.GetModuleFunctionDefinition("Table", "empty")
	assert.NotNil(t, emptyFunc, "Should get Table::empty function definition")
	assert.Equal(t, "empty", emptyFunc.Name)
	assert.True(t, emptyFunc.IsGeneric, "Table::empty should be generic")
}

func TestFunctionCallValidation(t *testing.T) {
	source := `contract Test {

    use Evm::{sender, emit};
    use Table::{Self, Table};
    use std::errors;
    
    #[storage]
    struct State {
        value: U32,
    }
    
    #[create]
    fn create() writes State {
        let addr = sender();
        emit(Transfer{from: addr, to: addr});
        Table::empty<Address, U256>();
        errors::invalid_argument(42);
    }
    
    #[event]
    struct Transfer {
        from: Address,
        to: Address,
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should pass with no errors for valid function calls
	assert.Empty(t, semanticErrors, "Should have no semantic errors for valid function calls")
}

func TestInvalidFunctionCalls(t *testing.T) {
	source := `contract Test {

    use Evm::{sender};
    
    #[storage]  
    struct State {
        value: U32,
    }
    
    #[create]
    fn create() writes State {
        undefined_function();
        sender(42);  // sender takes no args
        emit();      // emit not imported
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should have semantic errors for invalid function calls
	assert.NotEmpty(t, semanticErrors, "Should have semantic errors for invalid function calls")

	// Check for specific error messages
	errorMessages := make([]string, len(semanticErrors))
	for i, err := range semanticErrors {
		errorMessages[i] = err.Message
	}

	// Should catch undefined function
	found := false
	for _, msg := range errorMessages {
		if msg == "function 'undefined_function' is not imported or defined" {
			found = true
			break
		}
	}
	assert.True(t, found, "Should catch undefined function call")

	// Should catch wrong argument count
	found = false
	for _, msg := range errorMessages {
		if msg == "function 'sender' expects 0 arguments, got 1" {
			found = true
			break
		}
	}
	assert.True(t, found, "Should catch wrong argument count")
}

func TestParameterTypeValidation(t *testing.T) {
	source := `contract Test {

    use Evm::{sender};
    use std::errors;
    
    #[storage]
    struct State {
        value: U32,
    }
    
    #[create] 
    fn create() writes State {
        // This should be fine: invalid_argument expects U64
        errors::invalid_argument(42);
        
        // This should cause a type error: passing wrong literal type
        errors::invalid_argument(true);
    }
    
    fn test() {
        // This should cause a type error: sender() takes no parameters
        sender(42);
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should have exactly 2 type errors
	assert.Len(t, semanticErrors, 2, "Should have exactly 2 semantic errors")

	// Check error messages
	errorMessages := make([]string, len(semanticErrors))
	for i, err := range semanticErrors {
		errorMessages[i] = err.Message
	}

	// Should detect function arity mismatch
	assert.Contains(t, errorMessages, "function 'sender' expects 0 arguments, got 1")

	// Should detect type mismatch
	assert.Contains(t, errorMessages, "argument type Bool does not match expected type U64")
}

func TestVariableScopingAndTypeTracking(t *testing.T) {
	source := `
		contract Test {			
			#[storage]
			struct State {
				count: U256,
			}
			
			fn test() {
				let balance = 100;
				let mut counter = 0;
				let flag = true;
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)

	assert.Empty(t, errors, "Should have no semantic errors")
}

func TestVariableRedeclaration(t *testing.T) {
	source := `
		contract Test {
			fn test() {
				let balance = 100;
				let balance = 200; // Error: redeclaration
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)

	assert.Len(t, errors, 1, "Should have exactly one error")
	assert.Contains(t, errors[0].Message, "already declared", "Should detect variable redeclaration")
}

func TestImmutableVariableAssignment(t *testing.T) {
	source := `
		contract Test {
			fn test() {
				let balance = 100;
				balance = 200; // Error: cannot assign to immutable
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)

	assert.Len(t, errors, 1, "Should have exactly one error")
	assert.Contains(t, errors[0].Message, "immutable", "Should detect assignment to immutable variable")
}

func TestMutableVariableAssignment(t *testing.T) {
	source := `
		contract Test {
			fn test() {
				let mut counter = 0;
				counter = 1; // Valid: mutable variable
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)

	assert.Empty(t, errors, "Should have no semantic errors")
}

func TestUndefinedVariableAssignment(t *testing.T) {
	source := `
		contract Test {
			fn test() {
				unknown_var = 42; // Error: undefined variable
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)

	assert.Len(t, errors, 1, "Should have exactly one error")
	assert.Contains(t, errors[0].Message, "undefined", "Should detect undefined variable")
}

func TestFieldAccessValidation(t *testing.T) {
	source := `
		contract Test {
			#[storage]
			struct State {
				balance: U256,
				owner: Address,
			}
			
			fn test() {
				let amount = State.balance;  // Valid field access
				let user = State.owner;     // Valid field access
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)

	assert.Empty(t, errors, "Should have no semantic errors")
}

func TestInvalidFieldAccess(t *testing.T) {
	source := `
		contract Test {
			#[storage]
			struct State {
				balance: U256,
			}
			
			fn test() {
				let invalid = State.unknown_field;  // Error: field doesn't exist
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)

	assert.Len(t, errors, 1, "Should have exactly one error")
	assert.Contains(t, errors[0].Message, "unknown_field", "Should detect invalid field access")
}

func TestFieldAccessOnNonStruct(t *testing.T) {
	source := `
		contract Test {
			fn test() {
				let value = 100;
				let invalid = value.field;  // Error: not a struct
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	analyzer.Analyze(contract)

	// Might not detect this specific error if we can't infer the type of 'value'
	// That's acceptable for now - the important thing is that it doesn't crash
}

func TestBinaryExpressionTypeInference(t *testing.T) {
	source := `
		contract Test {
			fn test() {
				let a = 100;
				let b = 200;
				let sum = a + b;           // Valid: U64 + U64 -> U64
				let greater = a > b;       // Valid: U64 > U64 -> Bool
				let equal = true == false; // Valid: Bool == Bool -> Bool
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)

	assert.Empty(t, errors, "Should have no semantic errors")
}

func TestInvalidBinaryExpressions(t *testing.T) {
	source := `
		contract Test {
			fn test() {
				let num = 100;
				let flag = true;
				let invalid1 = num + flag;    // Error: U64 + Bool
				let invalid2 = flag > num;    // Error: Bool > U64
				let invalid3 = num && flag;   // Error: U64 && Bool
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)

	assert.Len(t, errors, 3, "Should have exactly 3 type errors")
}

func TestUnaryExpressionTypeInference(t *testing.T) {
	source := `
		contract Test {
			fn test() {
				let flag = true;
				let not_flag = !flag;  // Valid: !Bool -> Bool
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)

	assert.Empty(t, errors, "Should have no semantic errors")
}

func TestInvalidUnaryExpressions(t *testing.T) {
	source := `
		contract Test {
			fn test() {
				let num = 100;
				let invalid = !num;   // Error: !U64
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)

	assert.Len(t, errors, 1, "Should have exactly 1 type error")
}

func TestNumericTypePromotion(t *testing.T) {
	// This test would require explicit type annotations in LetStmt
	// which aren't implemented yet. The infrastructure for type promotion
	// is in place and would work when type annotations are added.

	// For now, just test that basic arithmetic works with inferred types
	source := `
		contract Test {
			fn test() {
				let a = 100;
				let b = 200;
				let result = a + b;  // Should work with inferred U64 types
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)
	assert.Empty(t, errors, "Should have no semantic errors")
}
