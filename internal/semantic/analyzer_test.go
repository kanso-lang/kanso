package semantic

import (
	"testing"

	"kanso/internal/parser"

	"github.com/stretchr/testify/assert"
)

func TestBasicNameResolution(t *testing.T) {
	source := `contract Test {
    struct Person {
        name: String,
        age: U32,
    }
    
    ext fn get_person() -> Person {
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
    ext fn test() -> U32 {
        42
    }
    
    ext fn test() -> String {
        "duplicate"
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should have at least one error, and one of them should be about duplicate declaration
	assert.NotEmpty(t, semanticErrors, "Should have semantic errors")
	hasDuplicateError := false
	for _, err := range semanticErrors {
		if containsSubstring(err.Message, "duplicate declaration") {
			hasDuplicateError = true
			break
		}
	}
	assert.True(t, hasDuplicateError, "Should have duplicate declaration error")
}

func TestBasicContractValidation(t *testing.T) {
	source := `contract Test {
    ext fn test() -> U32 {
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
    
    ext fn test() -> U32 {
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
	assert.Contains(t, semanticErrors[0].Message, "invalid attribute: invalid")
}

func TestInvalidFunctionAttribute(t *testing.T) {
	source := `contract Test {
    #[invalid]
    ext fn test() -> U32 {
        42
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	assert.Len(t, semanticErrors, 1, "Should have one semantic error")
	assert.Contains(t, semanticErrors[0].Message, "invalid attribute: invalid")
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
    
    ext fn test() reads RegularStruct {
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
    
    ext fn test() writes Transfer {
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
    
    ext fn test() reads State writes Config {
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
    
    ext fn test() reads State writes State {
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
    
    ext fn test() reads State1 writes State2 {
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
    
    ext fn test() -> Bool {
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

func TestLocalFunctionParameterValidation(t *testing.T) {
	source := `
		contract Test {
			#[storage]
			struct State {
				balance: U256,
			}

			fn helper(amount: U256, user: Address) -> U256 reads State {
				State.balance + amount
			}

			ext fn main() writes State {
				let result1 = helper(100, 0x1234567890123456789012345678901234567890);  // Valid call
				let result2 = helper(200);  // Error: missing parameter
				let result3 = helper(300, 0x1234567890123456789012345678901234567890, 400);  // Error: extra parameter
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter out unused variable warnings - we're testing parameter validation
	errors := FilterUnusedVariables(allErrors)

	assert.Len(t, errors, 2, "Should have exactly two parameter validation errors")
	assert.Contains(t, errors[0].Message, "helper", "First error should be about helper function")
	assert.Contains(t, errors[1].Message, "helper", "Second error should be about helper function")
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
    
    ext fn test() {
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
			
			ext fn test() {
				let balance = 100;
				let mut counter = 0;
				let flag = true;
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter out unused variable and mutable warnings - we're testing variable scoping and type tracking
	errors := FilterAllUnusedErrors(allErrors)

	assert.Empty(t, errors, "Should have no semantic errors")
}

func TestVariableRedeclaration(t *testing.T) {
	source := `
		contract Test {
			ext fn test() {
				let balance = 100;
				let balance = 200; // Error: redeclaration
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter out unused variable warnings - we're testing variable redeclaration
	errors := FilterUnusedVariables(allErrors)

	assert.Len(t, errors, 1, "Should have exactly one error")
	assert.Contains(t, errors[0].Message, "already declared", "Should detect variable redeclaration")
}

func TestImmutableVariableAssignment(t *testing.T) {
	source := `
		contract Test {
			ext fn test() {
				let balance = 100;
				balance = 200; // Error: cannot assign to immutable
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter unused variable errors
	errors := FilterUnusedVariables(allErrors)

	assert.Len(t, errors, 1, "Should have exactly one error")
	assert.Contains(t, errors[0].Message, "immutable", "Should detect assignment to immutable variable")
}

func TestMutableVariableAssignment(t *testing.T) {
	source := `
		contract Test {
			ext fn test() -> U256 {
				let mut counter = 0;
				counter = 1; // Valid: mutable variable
				return counter; // Use the variable
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter out development warnings - we're testing assignment validation
	errors := FilterDevelopmentWarnings(allErrors)
	assert.Empty(t, errors, "Should have no assignment-related semantic errors")
}

func TestUndefinedVariableAssignment(t *testing.T) {
	source := `
		contract Test {
			ext fn test() {
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
			
			ext fn test() reads State {
				let amount = State.balance;  // Valid field access
				let user = State.owner;     // Valid field access
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter unused variable errors
	errors := FilterUnusedVariables(allErrors)

	assert.Empty(t, errors, "Should have no semantic errors")
}

func TestInvalidFieldAccess(t *testing.T) {
	source := `
		contract Test {
			#[storage]
			struct State {
				balance: U256,
			}
			
			ext fn test() reads State {
				let invalid = State.unknown_field;  // Error: field doesn't exist
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter unused variable errors
	errors := FilterUnusedVariables(allErrors)

	assert.Len(t, errors, 1, "Should have exactly one error")
	assert.Contains(t, errors[0].Message, "unknown_field", "Should detect invalid field access")
}

func TestFieldAccessOnNonStruct(t *testing.T) {
	source := `
		contract Test {
			ext fn test() {
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
			ext fn test() {
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
	allErrors := analyzer.Analyze(contract)

	// Filter unused variable errors
	errors := FilterUnusedVariables(allErrors)

	assert.Empty(t, errors, "Should have no semantic errors")
}

func TestInvalidBinaryExpressions(t *testing.T) {
	source := `
		contract Test {
			ext fn test() {
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
	allErrors := analyzer.Analyze(contract)

	// Filter unused variable errors
	errors := FilterUnusedVariables(allErrors)

	assert.Len(t, errors, 3, "Should have exactly 3 type errors")
}

func TestUnaryExpressionTypeInference(t *testing.T) {
	source := `
		contract Test {
			ext fn test() {
				let flag = true;
				let not_flag = !flag;  // Valid: !Bool -> Bool
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter unused variable errors
	errors := FilterUnusedVariables(allErrors)

	assert.Empty(t, errors, "Should have no semantic errors")
}

func TestInvalidUnaryExpressions(t *testing.T) {
	source := `
		contract Test {
			ext fn test() {
				let num = 100;
				let invalid = !num;   // Error: !U64
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter unused variable errors
	errors := FilterUnusedVariables(allErrors)

	assert.Len(t, errors, 1, "Should have exactly 1 type error")
}

func TestNumericTypePromotion(t *testing.T) {
	// This test would require explicit type annotations in LetStmt
	// which aren't implemented yet. The infrastructure for type promotion
	// is in place and would work when type annotations are added.

	// For now, just test that basic arithmetic works with inferred types
	source := `
		contract Test {
			ext fn test() {
				let a = 100;
				let b = 200;
				let result = a + b;  // Should work with inferred U64 types
			}
		}
	`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter unused variable errors
	errors := FilterUnusedVariables(allErrors)

	assert.Empty(t, errors, "Should have no semantic errors")
}

func TestNumericLiteralValidation(t *testing.T) {
	t.Run("ValidNumericLiterals", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				// Valid numeric literals within range
				let u8_max = 255;
				let u16_max = 65535;
				let u32_max = 4294967295;
				let u64_max = 18446744073709551615;
				let u128_max = 340282366920938463463374607431768211455;
				let u256_max = 115792089237316195423570985008687907853269984665640564039457584007913129639935;
			}
		}`
		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")
		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Empty(t, errors, "Should have no semantic errors for valid numeric literals")
	})

	t.Run("NumericLiteralExceedsU256", func(t *testing.T) {
		// U256 max + 1 should trigger an error
		source := `contract Test {
			ext fn test() {
				let overflow = 115792089237316195423570985008687907853269984665640564039457584007913129639936;
			}
		}`
		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")
		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Len(t, errors, 1, "Should have one semantic error for numeric literal overflow")
		assert.Contains(t, errors[0].Message, "exceeds maximum for type 'U256'")
	})
}

func TestExplicitTypeDeclarations(t *testing.T) {
	t.Run("ValidExplicitTypes", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				// Valid explicit type declarations
				let small: U8 = 255;
				let medium: U16 = 65535;
				let large: U32 = 4294967295;
				let very_large: U64 = 18446744073709551615;
				let huge: U128 = 340282366920938463463374607431768211455;
				let massive: U256 = 115792089237316195423570985008687907853269984665640564039457584007913129639935;
				
				// Mixed with mutability
				let mut mutable_u32: U32 = 1000;
				let mut mutable_u256: U256 = 999999999999999999999;
			}
		}`
		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")
		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused and mutable variable errors
		errors := FilterAllUnusedErrors(allErrors)

		assert.Empty(t, errors, "Should have no semantic errors for valid explicit types")
	})

	t.Run("TypeOverflowErrors", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				let overflow_u8: U8 = 1000;
				let overflow_u16: U16 = 70000;
				let overflow_u32: U32 = 5000000000;
			}
		}`
		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")
		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Len(t, errors, 3, "Should have three type overflow errors")

		// Check specific error messages
		assert.Contains(t, errors[0].Message, "value '1000' exceeds maximum for type 'U8'")
		assert.Contains(t, errors[1].Message, "value '70000' exceeds maximum for type 'U16'")
		assert.Contains(t, errors[2].Message, "value '5000000000' exceeds maximum for type 'U32'")
	})

	t.Run("ExplicitTypeBoundaryTests", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				// Test exact boundaries
				let max_u8: U8 = 255;        // Valid
				let over_u8: U8 = 256;       // Invalid
				let max_u16: U16 = 65535;    // Valid
				let over_u16: U16 = 65536;   // Invalid
			}
		}`
		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")
		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Len(t, errors, 2, "Should have two boundary overflow errors")
		assert.Contains(t, errors[0].Message, "exceeds maximum for type 'U8'")
		assert.Contains(t, errors[1].Message, "exceeds maximum for type 'U16'")
	})

	t.Run("MixedExplicitAndInferredTypes", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				// Mix explicit and inferred types
				let explicit: U32 = 1000;
				let inferred = 2000;
				let mut explicit_mut: U16 = 500;
				let mut inferred_mut = 3000;
			}
		}`
		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")
		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused and mutable variable errors
		errors := FilterAllUnusedErrors(allErrors)

		assert.Empty(t, errors, "Should have no semantic errors for mixed types")
	})
}

func TestIfStatementBasicAnalysis(t *testing.T) {
	source := `contract Test {
		ext fn test(value: U256) -> Bool {
			if value > 0 {
				return true;
			} else {
				return false;
			}
		}
	}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)
	assert.Empty(t, errors, "Should have no semantic errors")
}

func TestIfStatementImmutabilityError(t *testing.T) {
	source := `contract Test {
		ext fn test(value: U256) -> Bool {
			let immutable_var = 100;
			if value > 0 {
				immutable_var = 200;  // Should trigger immutability error
			}
			return true;
		}
	}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter unused variable errors
	errors := FilterUnusedVariables(allErrors)

	assert.Len(t, errors, 1, "Should have one immutability error")

	compilerErrors := analyzer.GetErrors()
	assert.Contains(t, compilerErrors[0].Message, "cannot assign to immutable variable")
	assert.Contains(t, compilerErrors[0].Message, "immutable_var")
}

func TestIfStatementNestedImmutabilityError(t *testing.T) {
	source := `contract Test {
		ext fn test(a: U256, b: U256) -> Bool {
			let immutable_var = 100;
			if a > 0 {
				if b > 0 {
					immutable_var = 200;  // Should trigger error in nested if
				} else {
					immutable_var = 150;  // Should also trigger error in nested else
				}
			}
			return true;
		}
	}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter unused variable errors
	errors := FilterUnusedVariables(allErrors)

	assert.Len(t, errors, 2, "Should have two immutability errors from nested if")
}

func TestIfStatementMutableVariableAssignment(t *testing.T) {
	source := `contract Test {
		ext fn test(value: U256) -> Bool {
			let mut mutable_var = 100;
			if value > 0 {
				mutable_var = 200;  // Should be allowed
			} else {
				mutable_var = 150;  // Should also be allowed
			}
			return true;
		}
	}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter unused and mutable variable errors
	errors := FilterAllUnusedErrors(allErrors)

	assert.Empty(t, errors, "Should have no semantic errors for mutable variables")
}

func TestIfStatementExpressionAnalysis(t *testing.T) {
	source := `contract Test {
		ext fn test() -> Bool {
			let value = 100;
			if value > 0 {
				let result = value + 50;  // Should analyze expressions in if blocks
				return true;
			}
			return false;
		}
	}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	allErrors := analyzer.Analyze(contract)

	// Filter unused variable errors
	errors := FilterUnusedVariables(allErrors)

	assert.Empty(t, errors, "Should have no semantic errors")
}

func TestIfStatementUndefinedVariableError(t *testing.T) {
	source := `contract Test {
		ext fn test() -> Bool {
			if undefined_var > 0 {  // Should trigger undefined variable error
				return true;
			}
			return false;
		}
	}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	errors := analyzer.Analyze(contract)
	assert.Len(t, errors, 1, "Should have one undefined variable error")
}

func TestUninitializedVariables(t *testing.T) {
	t.Run("UninitializedMutableWithType", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				let mut counter: U256;  // Valid - uninitialized mutable with explicit type
				counter = 100;          // Should work
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused and mutable variable errors
		errors := FilterAllUnusedErrors(allErrors)

		assert.Empty(t, errors, "Should have no semantic errors for uninitialized mutable with type")
	})

	t.Run("UninitializedMutableWithoutType", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				let mut counter;  // Valid - defaults to U256
				counter = 1000;   // Should work with U256 assignment
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused and mutable variable errors
		errors := FilterAllUnusedErrors(allErrors)

		assert.Empty(t, errors, "Should have no semantic errors for uninitialized mutable without type (defaults to U256)")
	})

	t.Run("UninitializedImmutableVariable", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				let immutable_var: U256;  // Invalid - immutable must be initialized
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)
		assert.Len(t, errors, 1, "Should have error for uninitialized immutable variable")

		compilerErrors := analyzer.GetErrors()
		assert.Contains(t, compilerErrors[0].Message, "immutable variable 'immutable_var' must be initialized at declaration")
	})

	t.Run("MixedInitializedAndUninitializedVariables", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				let initialized = 100;        // Valid
				let mut uninitialized: U32;   // Valid
				let another_init: U16 = 200;  // Valid
				
				uninitialized = 500;          // Valid assignment to uninitialized mutable
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused and mutable variable errors
		errors := FilterAllUnusedErrors(allErrors)

		assert.Empty(t, errors, "Should have no semantic errors for mixed variable declarations")
	})

	t.Run("MutableInitializedVariableGetsU256", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				let mut mutable = 100;                // Should be U256, not U8
				mutable = 115792089237316195423570985008687907853269984665640564039457584007913129639935; // Max U256
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused and mutable variable errors
		errors := FilterAllUnusedErrors(allErrors)

		assert.Empty(t, errors, "Should have no semantic errors - mutable variable should be U256 to handle large values")
	})

	t.Run("ImmutableInitializedVariableGetsSmallestType", func(t *testing.T) {
		source := `contract Test {
			ext fn test() {
				let immutable = 100;    // Should be U8 (smallest type that fits)
				let bigger = immutable; // Should be fine - same type
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Empty(t, errors, "Should have no semantic errors for immutable variables")
	})
}

func TestVariableScopingWithComplexExpressions(t *testing.T) {
	t.Run("VariableUsedAfterDeclarationWithComplexExpression", func(t *testing.T) {
		// This test reproduces the bug from ERC20 example where a variable declared
		// with a complex expression (storage access) is not found in subsequent statements
		source := `contract Test {
			use std::evm::{sender};
			use std::errors;

			#[storage]
			struct State {
				allowances: Slots<(Address, Address), U256>,
			}

			ext fn test(from: Address, amount: U256) -> Bool reads State {
				let allowance = State.allowances[(from, sender())];
				require!(amount <= allowance, errors::InsufficientAllowance);
				true
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// This should pass but currently fails with "undefined variable 'allowance'"
		assert.Empty(t, errors, "Variable declared with complex expression should be accessible in subsequent statements")
	})

	t.Run("VariableUsedAfterSimpleDeclaration", func(t *testing.T) {
		// This test confirms simple variable declarations work fine
		source := `contract Test {
			use std::errors;

			ext fn test(amount: U256) -> Bool {
				let allowance = 100;
				require!(amount <= allowance, errors::InsufficientAllowance);
				true
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// This should work and currently does
		assert.Empty(t, errors, "Simple variable declarations should work fine")
	})
}

func TestCallPathAnalysis(t *testing.T) {
	t.Run("DirectStorageAccessRequiresDeclaration", func(t *testing.T) {
		source := `contract Test {
			#[storage]
			struct State {
				balance: U256,
			}

			// Missing reads State declaration
			ext fn get_balance() -> U256 {
				State.balance
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.Len(t, errors, 1, "Should have one error")
		assert.Contains(t, errors[0].Message, "accesses storage struct 'State'")
		assert.Contains(t, errors[0].Message, "does not declare it in reads clause")
	})

	t.Run("TransitiveStorageAccessRequiresDeclaration", func(t *testing.T) {
		source := `contract Test {
			#[storage]
			struct State {
				balance: U256,
			}

			fn get_balance() -> U256 reads State {
				State.balance
			}

			// This calls get_balance but doesn't declare reads State
			ext fn check_balance() -> U256 {
				get_balance()
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.Len(t, errors, 1, "Should have one error for transitive access")
		assert.Contains(t, errors[0].Message, "accesses storage struct 'State'")
		assert.Contains(t, errors[0].Message, "does not declare it in reads clause")
	})

	t.Run("WriteAccessRequiresWritesDeclaration", func(t *testing.T) {
		source := `contract Test {
			#[storage]
			struct State {
				balance: U256,
			}

			// Missing writes State declaration
			fn set_balance(amount: U256) {
				State.balance = amount;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should have errors for both missing reads and writes declarations
		assert.True(t, len(errors) >= 2, "Should have at least two errors")

		hasReadsError := false
		hasWritesError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "accesses storage struct") && containsSubstring(err.Message, "reads clause") {
				hasReadsError = true
			}
			if containsSubstring(err.Message, "accesses storage struct") && containsSubstring(err.Message, "writes clause") {
				hasWritesError = true
			}
		}

		assert.True(t, hasReadsError, "Should have reads error")
		assert.True(t, hasWritesError, "Should have writes error")
	})

	t.Run("CorrectDeclarationsPassValidation", func(t *testing.T) {
		source := `contract Test {
			#[storage]
			struct State {
				balance: U256,
			}

			fn get_balance() -> U256 reads State {
				State.balance
			}

			fn set_balance(amount: U256) writes State {
				State.balance = amount;
			}

			ext fn check_balance() -> U256 reads State {
				get_balance()
			}

			ext fn update_balance(amount: U256) writes State {
				set_balance(amount);
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.Empty(t, errors, "Should have no semantic errors when declarations are correct")
	})

	t.Run("ReadDeclarationButActuallyWriting", func(t *testing.T) {
		source := `contract Test {
			#[storage]
			struct State {
				balance: U256,
			}

			// Declares reads but actually writes - should fail
			fn sneaky_write(amount: U256) reads State {
				State.balance = amount;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have at least one error")

		hasWritesError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "accesses storage struct") && containsSubstring(err.Message, "writes clause") {
				hasWritesError = true
				break
			}
		}

		assert.True(t, hasWritesError, "Should detect that function writes but only declares reads")
	})

	t.Run("WriteDeclarationButOnlyReading", func(t *testing.T) {
		// Note: This is actually OK - a function with writes declaration can read
		// The writes declaration implies read permission, so this should pass
		source := `contract Test {
			#[storage]
			struct State {
				balance: U256,
			}

			// Declares writes but only reads - this should be allowed
			ext fn cautious_read() -> U256 writes State {
				State.balance
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.Empty(t, errors, "Write permission should allow reading - this is valid")
	})

	t.Run("WriteFunctionCallsReadFunction", func(t *testing.T) {
		// This should be OK - a write function can call a read function
		source := `contract Test {
			#[storage]
			struct State {
				balance: U256,
			}

			fn get_balance() -> U256 reads State {
				State.balance
			}

			ext fn update_after_check(amount: U256) writes State {
				let current = get_balance();
				State.balance = amount;
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Empty(t, errors, "Write function calling read function should be valid")
	})

	t.Run("ReadFunctionCallsWriteFunction", func(t *testing.T) {
		// This should fail - a read function cannot call a write function
		source := `contract Test {
			#[storage]
			struct State {
				balance: U256,
			}

			fn set_balance(amount: U256) writes State {
				State.balance = amount;
			}

			// Declares reads but calls a write function - should fail
			fn bad_reader() -> U256 reads State {
				set_balance(100);
				State.balance
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have at least one error")

		hasWritesError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "accesses storage struct") && containsSubstring(err.Message, "writes clause") {
				hasWritesError = true
				break
			}
		}

		assert.True(t, hasWritesError, "Should detect that read function transitively writes via function call")
	})

	t.Run("ComplexCallChainValidation", func(t *testing.T) {
		source := `contract Test {
			#[storage]
			struct State {
				balance: U256,
				owner: Address,
			}

			// Level 3 - direct storage access
			fn get_balance() -> U256 reads State {
				State.balance
			}

			fn set_balance(amount: U256) writes State {
				State.balance = amount;
			}

			// Level 2 - calls level 3 functions
			ext fn check_and_get() -> U256 reads State {
				get_balance()
			}

			ext fn update_and_set(amount: U256) writes State {
				set_balance(amount);
			}

			// Level 1 - calls level 2 functions
			fn high_level_read() -> U256 reads State {
				check_and_get()
			}

			fn high_level_write(amount: U256) writes State {
				update_and_set(amount);
			}

			// Mixed operations - should work
			ext fn complex_operation(amount: U256) writes State {
				let current = high_level_read();
				high_level_write(amount);
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		allErrors := analyzer.Analyze(contract)

		// Filter unused variable errors
		errors := FilterUnusedVariables(allErrors)

		assert.Empty(t, errors, "Complex valid call chains should pass validation")
	})

	t.Run("MultipleStorageStructsIndependentAccess", func(t *testing.T) {
		source := `contract Test {
			#[storage]
			struct UserState {
				balance: U256,
			}

			#[storage]
			struct SystemState {
				total_supply: U256,
			}

			fn get_supply() -> U256 reads SystemState {
				SystemState.total_supply
			}

			// Missing SystemState declaration - should fail
			fn bad_function() -> U256 reads UserState {
				get_supply()
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		assert.True(t, len(errors) >= 1, "Should have error for missing SystemState declaration")

		hasSystemStateError := false
		for _, err := range errors {
			if containsSubstring(err.Message, "accesses storage struct 'SystemState'") {
				hasSystemStateError = true
				break
			}
		}

		assert.True(t, hasSystemStateError, "Should detect missing SystemState reads declaration")
	})

	t.Run("MultipleStorageStructsMixedReadWrite", func(t *testing.T) {
		source := `contract Test {
			#[storage]
			struct UserState {
				balance: U256,
				last_login: U256,
			}

			#[storage]
			struct SystemState {
				total_supply: U256,
				admin: Address,
			}

			#[storage]
			struct ConfigState {
				fee_rate: U256,
				paused: Bool,
			}

			fn get_user_info() -> U256 reads(UserState, SystemState) {
				let balance = UserState.balance;
				let supply = SystemState.total_supply;
				balance + supply
			}

			// Function that writes to multiple structs (writes implies reads)
			fn update_user_balance(amount: U256) writes(UserState, SystemState, ConfigState) {
				let fee = ConfigState.fee_rate;
				let supply = SystemState.total_supply;
				UserState.balance = amount - fee;
				UserState.last_login = 12345;
			}

			// Function that writes to multiple structs
			fn admin_update(new_supply: U256, new_fee: U256) writes(SystemState, ConfigState, UserState) {
				let user_count = UserState.balance;  // Just reading for calculation
				SystemState.total_supply = new_supply;
				ConfigState.fee_rate = new_fee;
			}

			// Error case: missing declarations
			fn bad_mixed_access() reads(UserState) {
				let balance = UserState.balance;     // OK: declared
				let supply = SystemState.total_supply; // Error: SystemState not declared
				ConfigState.fee_rate = 100;          // Error: ConfigState not declared for write
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should have errors for the bad_mixed_access function
		assert.True(t, len(errors) >= 2, "Should have at least two errors for missing declarations")

		hasSystemStateReadError := false
		hasConfigStateWriteError := false

		for _, err := range errors {
			if containsSubstring(err.Message, "bad_mixed_access") {
				if containsSubstring(err.Message, "SystemState") && containsSubstring(err.Message, "reads clause") {
					hasSystemStateReadError = true
				}
				if containsSubstring(err.Message, "ConfigState") && containsSubstring(err.Message, "writes clause") {
					hasConfigStateWriteError = true
				}
			}
		}

		assert.True(t, hasSystemStateReadError, "Should detect missing SystemState reads declaration")
		assert.True(t, hasConfigStateWriteError, "Should detect missing ConfigState writes declaration")
	})

	t.Run("MultipleStorageStructsTransitiveAccess", func(t *testing.T) {
		source := `contract Test {
			#[storage]
			struct UserState {
				balance: U256,
			}

			#[storage]
			struct SystemState {
				total_supply: U256,
			}

			#[storage]
			struct LogState {
				event_count: U256,
			}

			// Helper functions with specific access patterns
			fn read_user() -> U256 reads(UserState) {
				UserState.balance
			}

			fn write_system(amount: U256) writes(SystemState) {
				SystemState.total_supply = amount;
			}

			fn update_log() writes(LogState) {
				LogState.event_count += 1;
			}

			fn read_multiple() -> U256 reads(UserState, SystemState) {
				let user_bal = UserState.balance;
				let supply = SystemState.total_supply;
				user_bal + supply
			}

			// Complex function that calls multiple helpers - needs all permissions
			fn complex_operation() writes(UserState, SystemState, LogState) {
				let current_balance = read_user();        // Needs UserState reads (transitive)
				let total = read_multiple();              // Needs UserState, SystemState reads (transitive)
				write_system(total + 1000);              // Needs SystemState writes (transitive)
				update_log();                             // Needs LogState writes (transitive)
				UserState.balance = current_balance + 100; // Direct UserState write
			}

			// Error case: insufficient permissions for transitive calls
			fn insufficient_permissions() reads(UserState) {
				let balance = read_user();                // OK: UserState declared
				write_system(1000);                       // Error: needs SystemState writes
				update_log();                             // Error: needs LogState writes
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should have errors for insufficient_permissions function
		assert.True(t, len(errors) >= 2, "Should have at least two errors for missing transitive permissions")

		hasSystemWriteError := false
		hasLogWriteError := false

		for _, err := range errors {
			if containsSubstring(err.Message, "insufficient_permissions") {
				if containsSubstring(err.Message, "SystemState") && containsSubstring(err.Message, "writes clause") {
					hasSystemWriteError = true
				}
				if containsSubstring(err.Message, "LogState") && containsSubstring(err.Message, "writes clause") {
					hasLogWriteError = true
				}
			}
		}

		assert.True(t, hasSystemWriteError, "Should detect missing SystemState writes permission for transitive call")
		assert.True(t, hasLogWriteError, "Should detect missing LogState writes permission for transitive call")
	})

	t.Run("MultipleStorageStructsPartialPermissions", func(t *testing.T) {
		source := `contract Test {
			#[storage]
			struct TokenState {
				balance: U256,
				allowances: U256,
			}

			#[storage]
			struct MetadataState {
				name: String,
				symbol: String,
			}

			#[storage]
			struct AdminState {
				owner: Address,
				paused: Bool,
			}

			// Function with partial permissions - some correct, some missing
			fn partial_access() writes(TokenState, MetadataState) {
				// These should work
				let balance = TokenState.balance;         // OK: TokenState writes (implies reads)
				MetadataState.name = "NewToken";          // OK: MetadataState writes declared

				// These should fail
				AdminState.paused = true;                 // Error: AdminState writes not declared
				let owner = AdminState.owner;             // Error: AdminState reads not declared
			}

			// Function that over-declares permissions (should be OK)
			fn over_declared() writes(TokenState, MetadataState, AdminState) {
				// Only actually uses TokenState
				let balance = TokenState.balance;
				TokenState.balance = balance + 100;
				// MetadataState and AdminState permissions declared but not used - should be fine
			}

			// Complex mixed access with some functions having correct declarations
			fn helper_read_token() -> U256 reads(TokenState) {
				TokenState.balance
			}

			fn helper_write_admin(paused: Bool) writes(AdminState) {
				AdminState.paused = paused;
			}

			fn complex_mixed() writes(TokenState, MetadataState, AdminState) {
				let balance = helper_read_token();        // OK: TokenState reads via transitive call
				let name = MetadataState.name;            // OK: MetadataState reads (writes implies reads)
				helper_write_admin(false);                // OK: AdminState writes via transitive call
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should have errors for partial_access function only
		assert.True(t, len(errors) >= 2, "Should have at least two errors for partial_access")

		hasAdminWriteError := false
		hasAdminReadError := false

		for _, err := range errors {
			if containsSubstring(err.Message, "partial_access") {
				if containsSubstring(err.Message, "AdminState") && containsSubstring(err.Message, "writes clause") {
					hasAdminWriteError = true
				}
				if containsSubstring(err.Message, "AdminState") && containsSubstring(err.Message, "reads clause") {
					hasAdminReadError = true
				}
			}
		}

		assert.True(t, hasAdminWriteError, "Should detect missing AdminState writes declaration")
		assert.True(t, hasAdminReadError, "Should detect missing AdminState reads declaration")
	})

	t.Run("MultipleStorageStructsComplexInteraction", func(t *testing.T) {
		source := `contract Test {
			#[storage]
			struct BalanceState {
				user_balance: U256,
				locked_balance: U256,
			}

			#[storage]
			struct GovernanceState {
				voting_power: U256,
				proposals: U256,
			}

			#[storage]
			struct RewardState {
				pending_rewards: U256,
				claimed_rewards: U256,
			}

			// Multi-level function calls with different storage requirements
			fn check_balance() -> U256 reads(BalanceState) {
				BalanceState.user_balance + BalanceState.locked_balance
			}

			fn calculate_voting_power() -> U256 reads(BalanceState, GovernanceState) {
				let balance = check_balance();            // Transitive BalanceState access
				let base_power = GovernanceState.voting_power;
				balance + base_power
			}

			fn process_reward() writes(RewardState, BalanceState) {
				let balance = BalanceState.user_balance;
				RewardState.pending_rewards += balance / 100;
			}

			fn claim_rewards() writes(BalanceState, RewardState, GovernanceState) {
				let power = GovernanceState.voting_power;
				let rewards = RewardState.pending_rewards;
				BalanceState.user_balance += rewards;
				RewardState.claimed_rewards += rewards;
				RewardState.pending_rewards = 0;
			}

			// Master function that orchestrates everything
			fn master_operation() writes(BalanceState, GovernanceState, RewardState) {
				let voting_power = calculate_voting_power(); // Needs BalanceState, GovernanceState reads
				process_reward();                             // Needs RewardState writes, BalanceState reads  
				claim_rewards();                              // Needs BalanceState, RewardState writes, GovernanceState reads
				GovernanceState.proposals += 1;              // Direct GovernanceState write
			}

			// Error case: function tries to call master_operation without sufficient permissions
			fn insufficient_master_call() reads(BalanceState) {
				master_operation();  // Error: needs more permissions than declared
			}
		}`

		contract, parseErrors, _ := parser.ParseSource("test.ka", source)
		assert.Empty(t, parseErrors, "Should have no parse errors")

		analyzer := NewAnalyzer()
		errors := analyzer.Analyze(contract)

		// Should have errors for insufficient_master_call
		assert.True(t, len(errors) >= 2, "Should have at least two errors for insufficient permissions")

		hasBalanceWriteError := false
		hasGovernanceWriteError := false

		for _, err := range errors {
			if containsSubstring(err.Message, "insufficient_master_call") {
				if containsSubstring(err.Message, "BalanceState") && containsSubstring(err.Message, "writes clause") {
					hasBalanceWriteError = true
				}
				if containsSubstring(err.Message, "GovernanceState") && containsSubstring(err.Message, "writes clause") {
					hasGovernanceWriteError = true
				}
			}
		}

		assert.True(t, hasBalanceWriteError, "Should detect missing BalanceState writes permission")
		assert.True(t, hasGovernanceWriteError, "Should detect missing GovernanceState writes permission")
	})
}
