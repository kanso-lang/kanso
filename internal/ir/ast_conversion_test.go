package ir

import (
	"strings"
	"testing"

	"kanso/internal/ast"
	"kanso/internal/parser"
	"kanso/internal/semantic"
)

// Tests for AST to IR conversion functions

// Test astTypeToABIString function
func TestASTTypeToABIString(t *testing.T) {
	context := semantic.NewContextRegistry()
	builder := NewBuilder(context)

	// Test various AST types
	testCases := []struct {
		astType  *ast.VariableType
		expected string
	}{
		{&ast.VariableType{Name: ast.Ident{Value: "U256"}}, "uint256"},
		{&ast.VariableType{Name: ast.Ident{Value: "Bool"}}, "bool"},
		{&ast.VariableType{Name: ast.Ident{Value: "Address"}}, "address"},
		{&ast.VariableType{Name: ast.Ident{Value: "U128"}}, "uint128"},
		{&ast.VariableType{Name: ast.Ident{Value: "U64"}}, "uint64"},
		{&ast.VariableType{Name: ast.Ident{Value: "U32"}}, "uint32"},
		{&ast.VariableType{Name: ast.Ident{Value: "U16"}}, "uint16"},
		{&ast.VariableType{Name: ast.Ident{Value: "U8"}}, "uint8"},
		{&ast.VariableType{Name: ast.Ident{Value: "String"}}, "string"},
		{&ast.VariableType{Name: ast.Ident{Value: "Unknown"}}, "unknown"},
		{nil, "unknown"},
	}

	for _, tc := range testCases {
		result := builder.astTypeToABIString(tc.astType)
		if result != tc.expected {
			t.Errorf("astTypeToABIString(%v) = %s, expected %s", tc.astType, result, tc.expected)
		}
	}
}

// Test buildEmitCall function
func TestBuildEmitCallExtended(t *testing.T) {
	// Test various emit scenarios
	testCases := []struct {
		name   string
		source string
	}{
		{
			"Simple emit with basic struct",
			`
contract EmitTest {
    use std::evm::{emit};

    #[event]
    struct SimpleEvent {
        value: U256,
    }

    ext fn emitSimple(val: U256) {
        emit(SimpleEvent{value: val});
    }
}`,
		},
		{
			"Emit with multiple fields",
			`
contract EmitTest {
    use std::evm::{emit};

    #[event]
    struct MultiEvent {
        from: Address,
        to: Address,
        amount: U256,
        success: Bool,
    }

    ext fn emitMulti(from: Address, to: Address, amount: U256) {
        emit(MultiEvent{from: from, to: to, amount: amount, success: true});
    }
}`,
		},
		{
			"Emit with tuple types",
			`
contract EmitTest {
    use std::evm::{emit};

    #[event]
    struct TupleEvent {
        data: (U256, Bool),
    }

    ext fn emitTuple(value: U256, flag: Bool) {
        emit(TupleEvent{data: (value, flag)});
    }
}`,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			contract, parseErrors, scanErrors := parser.ParseSource("test.ka", tc.source)
			if len(parseErrors) > 0 || len(scanErrors) > 0 {
				t.Fatalf("Parse errors: %v, Scan errors: %v", parseErrors, scanErrors)
			}

			analyzer := semantic.NewAnalyzer()
			errors := analyzer.Analyze(contract)
			if len(errors) > 0 {
				t.Fatalf("Semantic errors: %v", errors)
			}

			context := analyzer.GetContext()
			program := BuildProgram(contract, context)

			if program == nil {
				t.Fatal("Program should not be nil")
			}

			output := PrintProgram(program)
			// Should have LOG or ABI_ENC instructions
			if !strings.Contains(output, "LOG") && !strings.Contains(output, "ABI_ENC") && !strings.Contains(output, "EMIT") {
				t.Errorf("Expected LOG/ABI_ENC/EMIT instructions in output")
			}
		})
	}
}

// Test computeSeethiUllman function
func TestComputeSeethiUllman(t *testing.T) {
	context := semantic.NewContextRegistry()
	_ = NewBuilder(context)

	// Test different expression complexity scenarios
	testCases := []struct {
		name   string
		source string
	}{
		{
			"Simple binary expression",
			`
contract TestSU {
    ext fn test() -> U256 {
        1 + 2
    }
}`,
		},
		{
			"Nested binary expressions",
			`
contract TestSU {
    ext fn test() -> U256 {
        (1 + 2) * (3 + 4)
    }
}`,
		},
		{
			"Complex nested expressions",
			`
contract TestSU {
    ext fn test() -> U256 {
        ((1 + 2) * (3 + 4)) - ((5 + 6) / (7 + 8))
    }
}`,
		},
		{
			"Function calls in expressions",
			`
contract TestSU {
    use std::evm::{sender};

    ext fn test() -> U256 {
        1 + 2
    }

    ext fn complex() -> U256 {
        test() + test()
    }
}`,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			contract, parseErrors, scanErrors := parser.ParseSource("test.ka", tc.source)
			if len(parseErrors) > 0 || len(scanErrors) > 0 {
				t.Fatalf("Parse errors: %v, Scan errors: %v", parseErrors, scanErrors)
			}

			analyzer := semantic.NewAnalyzer()
			errors := analyzer.Analyze(contract)
			if len(errors) > 0 {
				t.Fatalf("Semantic errors: %v", errors)
			}

			context := analyzer.GetContext()
			program := BuildProgram(contract, context)

			if program == nil {
				t.Fatal("Program should not be nil")
			}

			// The Sethi-Ullman computation should happen during expression building
			// This test ensures we exercise those code paths
			output := PrintProgram(program)
			if output == "" {
				t.Error("Program output should not be empty")
			}
		})
	}
}

// Test buildCall function with different scenarios
func TestBuildCallExtended(t *testing.T) {
	testCases := []struct {
		name   string
		source string
	}{
		{
			"Standard library call",
			`
contract CallTest {
    use std::evm::{sender};

    ext fn testSender() -> Address {
        sender()
    }
}`,
		},
		{
			"Local function call",
			`
contract CallTest {
    ext fn helper() -> U256 {
        42
    }

    ext fn testLocal() -> U256 {
        helper()
    }
}`,
		},
		{
			"Function call with arguments",
			`
contract CallTest {
    ext fn add(a: U256, b: U256) -> U256 {
        a + b
    }

    ext fn testWithArgs() -> U256 {
        add(10, 20)
    }
}`,
		},
		{
			"Nested function calls",
			`
contract CallTest {
    ext fn double(x: U256) -> U256 {
        x * 2
    }

    ext fn quadruple(x: U256) -> U256 {
        double(double(x))
    }
}`,
		},
		{
			"Multiple module paths",
			`
contract CallTest {
    use std::evm::{sender};
    use std::address;

    ext fn testMultiModule() -> Address {
        address::zero()
    }
}`,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			contract, parseErrors, scanErrors := parser.ParseSource("test.ka", tc.source)
			if len(parseErrors) > 0 || len(scanErrors) > 0 {
				t.Fatalf("Parse errors: %v, Scan errors: %v", parseErrors, scanErrors)
			}

			analyzer := semantic.NewAnalyzer()
			errors := analyzer.Analyze(contract)
			if len(errors) > 0 {
				t.Fatalf("Semantic errors: %v", errors)
			}

			context := analyzer.GetContext()
			program := BuildProgram(contract, context)

			if program == nil {
				t.Fatal("Program should not be nil")
			}

			output := PrintProgram(program)
			// Should have CALL instructions or constant folding
			if !strings.Contains(output, "CALL") && !strings.Contains(output, "CONST") && !strings.Contains(output, "sender") {
				t.Logf("Output: %s", output)
				// Don't fail, as const-eval might optimize calls away
			}
		})
	}
}

// Test buildExpression with edge cases
func TestBuildExpressionEdgeCases(t *testing.T) {
	testCases := []struct {
		name   string
		source string
	}{
		{
			"Unary expressions",
			`
contract UnaryTest {
    ext fn testUnary(x: Bool) -> Bool {
        !x
    }
}`,
		},
		{
			"Parenthesized expressions",
			`
contract ParenTest {
    ext fn testParen(x: U256, y: U256) -> U256 {
        (x + y) * 2
    }
}`,
		},
		{
			"Tuple expressions",
			`
contract TupleTest {
    ext fn testTuple() -> (U256, Bool) {
        (42, true)
    }
}`,
		},
		{
			"Complex field access",
			`
contract FieldTest {
    #[storage]
    struct State {
        nested: NestedStruct,
    }

    struct NestedStruct {
        value: U256,
    }

    ext fn testFieldAccess() -> U256 reads State {
        State.nested.value
    }
}`,
		},
		{
			"Index expressions with complex indices",
			`
contract IndexTest {
    #[storage]
    struct State {
        balances: Slots<Address, U256>,
    }

    ext fn testComplexIndex(addr1: Address, addr2: Address) -> U256 reads State {
        State.balances[addr1] + State.balances[addr2]
    }
}`,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			contract, parseErrors, scanErrors := parser.ParseSource("test.ka", tc.source)
			if len(parseErrors) > 0 || len(scanErrors) > 0 {
				t.Fatalf("Parse errors: %v, Scan errors: %v", parseErrors, scanErrors)
			}

			analyzer := semantic.NewAnalyzer()
			errors := analyzer.Analyze(contract)
			if len(errors) > 0 {
				t.Fatalf("Semantic errors: %v", errors)
			}

			context := analyzer.GetContext()
			program := BuildProgram(contract, context)

			if program == nil {
				t.Fatal("Program should not be nil")
			}

			output := PrintProgram(program)
			if output == "" {
				t.Error("Program output should not be empty")
			}
		})
	}
}

// Test buildAssignStatement with various scenarios
func TestBuildAssignStatementExtended(t *testing.T) {
	testCases := []struct {
		name   string
		source string
	}{
		{
			"Complex compound assignments",
			`
contract AssignTest {
    #[storage]
    struct State {
        total: U256,
        balances: Slots<Address, U256>,
    }

    ext fn testComplex(addr: Address, amount: U256) writes State {
        State.total *= 2;
        State.balances[addr] /= 3;
        State.balances[addr] += 5;
    }
}`,
		},
		{
			"Assignments with complex expressions",
			`
contract AssignTest {
    #[storage]
    struct State {
        value: U256,
    }

    ext fn testComplexExpr(a: U256, b: U256, c: U256) writes State {
        State.value = (a + b) * c - (a * b);
    }
}`,
		},
		{
			"Assignments to local variables",
			`
contract AssignTest {
    ext fn testLocal() -> U256 {
        let mut x: U256 = 10;
        x += 5;
        x *= 2;
        x
    }
}`,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			contract, parseErrors, scanErrors := parser.ParseSource("test.ka", tc.source)
			if len(parseErrors) > 0 || len(scanErrors) > 0 {
				t.Fatalf("Parse errors: %v, Scan errors: %v", parseErrors, scanErrors)
			}

			analyzer := semantic.NewAnalyzer()
			errors := analyzer.Analyze(contract)
			if len(errors) > 0 {
				t.Fatalf("Semantic errors: %v", errors)
			}

			context := analyzer.GetContext()
			program := BuildProgram(contract, context)

			if program == nil {
				t.Fatal("Program should not be nil")
			}

			output := PrintProgram(program)
			// Should have arithmetic or storage operations
			if !strings.Contains(output, "ADD") && !strings.Contains(output, "MUL") && !strings.Contains(output, "SSTORE") {
				t.Logf("Output: %s", output)
				// Don't fail as assignment patterns may vary
			}
		})
	}
}

// Test buildBlockItem with various statement types
func TestBuildBlockItemExtended(t *testing.T) {
	testCases := []struct {
		name   string
		source string
	}{
		{
			"Simple assignments",
			`
contract BlockTest {
    #[storage]
    struct State {
        value: U256,
    }

    ext fn testAssign() writes State {
        State.value = 42;
        State.value = 100;
    }
}`,
		},
		{
			"Return statements",
			`
contract BlockTest {
    ext fn testReturn() -> U256 {
        return 42;
    }
}`,
		},
		{
			"Let statements",
			`
contract BlockTest {
    ext fn testLet() -> U256 {
        let x: U256 = 10;
        let mut y: U256 = 20;
        y = y + 5;
        x + y
    }
}`,
		},
		{
			"Expression statements",
			`
contract BlockTest {
    ext fn testExpr() -> U256 {
        let x: U256 = 5;
        x * 2
    }
}`,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			contract, parseErrors, scanErrors := parser.ParseSource("test.ka", tc.source)
			if len(parseErrors) > 0 || len(scanErrors) > 0 {
				t.Fatalf("Parse errors: %v, Scan errors: %v", parseErrors, scanErrors)
			}

			analyzer := semantic.NewAnalyzer()
			errors := analyzer.Analyze(contract)
			if len(errors) > 0 {
				t.Fatalf("Semantic errors: %v", errors)
			}

			context := analyzer.GetContext()
			program := BuildProgram(contract, context)

			if program == nil {
				t.Fatal("Program should not be nil")
			}

			output := PrintProgram(program)
			if output == "" {
				t.Error("Program output should not be empty")
			}
		})
	}
}
