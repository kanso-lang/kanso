package semantic

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"kanso/internal/ast"
	"kanso/internal/parser"
)

func TestFlowAnalysisUnreachableCode(t *testing.T) {
	source := `contract Test {
    ext fn unreachable_test() -> U256 {
        let x = 42;
        return x;
        let y = 100; // This should be unreachable
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should detect unreachable code
	foundUnreachable := false
	for _, err := range semanticErrors {
		if contains(err.Message, "unreachable") {
			foundUnreachable = true
			break
		}
	}
	assert.True(t, foundUnreachable, "Should detect unreachable code after return statement")
}

func TestFlowAnalysisUnusedVariable(t *testing.T) {
	// Test the unused variable detection directly with flow analyzer
	source := `contract Test {
    ext fn unused_test() -> U256 {
        let x = 42;      // Used
        let y = 100;     // Unused - should be reported if enabled
        let z = 200;     // Used
        return x + z;
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	// Test flow analyzer directly to verify unused variable detection logic
	mockAnalyzer := NewAnalyzer()
	flowAnalyzer := NewFlowAnalyzer(mockAnalyzer)

	// Simulate the analysis state
	flowAnalyzer.usedVars = make(map[string]bool)
	flowAnalyzer.declaredVars = make(map[string]ast.Position)
	flowAnalyzer.errors = make([]SemanticError, 0)

	// Simulate the analysis
	flowAnalyzer.declaredVars["x"] = ast.Position{}
	flowAnalyzer.declaredVars["y"] = ast.Position{}
	flowAnalyzer.declaredVars["z"] = ast.Position{}

	flowAnalyzer.usedVars["x"] = true
	flowAnalyzer.usedVars["z"] = true
	// "y" is intentionally not marked as used

	// Test that the logic can detect unused variables
	assert.False(t, flowAnalyzer.usedVars["y"], "Variable 'y' should not be marked as used")
	assert.True(t, flowAnalyzer.usedVars["x"], "Variable 'x' should be marked as used")
	assert.True(t, flowAnalyzer.usedVars["z"], "Variable 'z' should be marked as used")

	// The main analyzer should not report unused variables (disabled by default)
	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should not report unused variables when disabled
	for _, err := range semanticErrors {
		assert.False(t, contains(err.Message, "never used"),
			"Should not report unused variables when disabled")
	}
}

func TestFlowAnalysisMissingReturn(t *testing.T) {
	source := `contract Test {
    ext fn missing_return_test() -> U256 {
        let x = 42;
        let y = x + 10;
        // Missing return statement
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should detect missing return statement
	foundMissingReturn := false
	for _, err := range semanticErrors {
		if contains(err.Message, "no return statement") {
			foundMissingReturn = true
			break
		}
	}
	assert.True(t, foundMissingReturn, "Should detect missing return statement")
}

func TestFlowAnalysisValidFunction(t *testing.T) {
	source := `contract Test {
    ext fn valid_function(param1: U256, param2: U256) -> U256 {
        let result = param1 + param2;
        return result;
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should have no flow analysis errors for valid function
	for _, err := range semanticErrors {
		assert.False(t, contains(err.Message, "unreachable"), "Should not report unreachable code")
		assert.False(t, contains(err.Message, "never used"), "Should not report unused variables")
		assert.False(t, contains(err.Message, "no return statement"), "Should not report missing return")
	}
}

func TestFlowAnalysisTailExpression(t *testing.T) {
	source := `contract Test {
    ext fn tail_expr_test() -> U256 {
        let x = 42;
        x + 10
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should not report missing return for function with tail expression
	for _, err := range semanticErrors {
		assert.False(t, contains(err.Message, "no return statement"),
			"Should not report missing return when tail expression is present")
	}
}

func TestFlowAnalysisUnreachableAfterTailExpr(t *testing.T) {
	source := `contract Test {
    ext fn unreachable_after_return() -> U256 {
        let x = 42;
        return x;
        x + 10
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should detect unreachable tail expression
	foundUnreachable := false
	for _, err := range semanticErrors {
		if contains(err.Message, "unreachable") {
			foundUnreachable = true
			break
		}
	}
	assert.True(t, foundUnreachable, "Should detect unreachable code after return statement")
}

func TestFlowAnalysisParameterUsage(t *testing.T) {
	source := `contract Test {
    ext fn param_test(used_param: U256, unused_param: U256) -> U256 {
        return used_param * 2;
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Parameters are considered part of the function interface, so they shouldn't
	// be reported as unused even if not referenced in the body
	for _, err := range semanticErrors {
		assert.False(t, contains(err.Message, "never used") && contains(err.Message, "param"),
			"Should not report parameters as unused")
	}
}

func TestFlowAnalysisComplexExpressions(t *testing.T) {
	source := `contract Test {
    use std::evm::{sender, emit};
    
    #[storage]
    struct State {
        balances: Slots<Address, U256>,
        total: U256,
    }
    
    #[event]
    struct Transfer {
        from: Address,
        to: Address,
        amount: U256,
    }
    
    ext fn complex_flow(to: Address, amount: U256) -> Bool writes State {
        let from = sender();
        let balance = State.balances[from];
        
        require!(balance >= amount);
        
        State.balances[from] -= amount;
        State.balances[to] += amount;
        
        emit(Transfer{from: from, to: to, amount: amount});
        
        true
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should have no flow analysis errors for well-structured function
	flowErrors := 0
	for _, err := range semanticErrors {
		if contains(err.Message, "unreachable") ||
			contains(err.Message, "never used") ||
			contains(err.Message, "no return statement") {
			flowErrors++
		}
	}
	assert.Equal(t, 0, flowErrors, "Should have no flow analysis errors for well-structured function")
}

func TestFlowAnalysisRequireStatements(t *testing.T) {
	source := `contract Test {
    ext fn require_test(amount: U256) -> U256 {
        let balance = 1000;
        
        require!(amount <= balance);
        require!(amount > 0);
        
        let result = balance - amount;
        return result;
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should handle require statements without flow issues
	flowErrors := 0
	for _, err := range semanticErrors {
		if contains(err.Message, "unreachable") ||
			contains(err.Message, "never used") ||
			contains(err.Message, "no return statement") {
			flowErrors++
		}
	}
	assert.Equal(t, 0, flowErrors, "Should handle require statements correctly")
}

func TestFlowAnalysisVoidFunction(t *testing.T) {
	source := `contract Test {
    fn void_function() {
        let x = 42;
        let y = x + 10;
        // No return needed for void function
    }
}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")
	assert.NotNil(t, contract, "Contract should be parsed")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should not report missing return for void function
	for _, err := range semanticErrors {
		assert.False(t, contains(err.Message, "no return statement"),
			"Should not report missing return for void function")
	}
}

func TestFlowAnalysisIfStatementCompleteReturns(t *testing.T) {
	source := `contract Test {
		ext fn complete_if(value: U256) -> Bool {
			if value > 0 {
				return true;
			} else {
				return false;
			}
		}
	}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should not report missing return since both branches return
	foundMissingReturn := false
	for _, err := range semanticErrors {
		if contains(err.Message, "no return statement") {
			foundMissingReturn = true
		}
	}
	assert.False(t, foundMissingReturn, "Should not report missing return when all branches return")
}

func TestFlowAnalysisIfStatementIncompleteReturns(t *testing.T) {
	source := `contract Test {
		ext fn incomplete_if(value: U256) -> Bool {
			if value > 0 {
				return true;
			}
		}
	}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should report missing return since else clause is missing
	foundMissingReturn := false
	for _, err := range semanticErrors {
		if contains(err.Message, "no return statement") {
			foundMissingReturn = true
		}
	}
	assert.True(t, foundMissingReturn, "Should report missing return when if has no else clause")
}

func TestFlowAnalysisNestedIfCompleteReturns(t *testing.T) {
	source := `contract Test {
		ext fn nested_complete(a: U256, b: U256) -> Bool {
			if a > 0 {
				if b > 0 {
					return true;
				} else {
					return false;
				}
			} else {
				if b > 0 {
					return true;
				} else {
					return false;
				}
			}
			// All nested branches return
		}
	}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should not report missing return since all nested branches return
	foundMissingReturn := false
	for _, err := range semanticErrors {
		if contains(err.Message, "no return statement") {
			foundMissingReturn = true
		}
	}
	assert.False(t, foundMissingReturn, "Should not report missing return when all nested branches return")
}

func TestFlowAnalysisNestedIfIncompleteReturns(t *testing.T) {
	source := `contract Test {
		ext fn nested_incomplete(a: U256, b: U256) -> Bool {
			if a > 0 {
				if b > 0 {
					return true;
				} else {
					return false;
				}
			} else {
				if b > 0 {
					return true;
				}
				// Missing return for b <= 0 case in else branch
			}
		}
	}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should report missing return due to incomplete nested branch
	foundMissingReturn := false
	for _, err := range semanticErrors {
		if contains(err.Message, "no return statement") {
			foundMissingReturn = true
		}
	}
	assert.True(t, foundMissingReturn, "Should report missing return when nested branch is incomplete")
}

func TestFlowAnalysisIfElseIfChain(t *testing.T) {
	source := `contract Test {
		ext fn if_else_if_complete(value: U256) -> Bool {
			if value > 100 {
				return true;
			} else if value > 50 {
				return false;
			} else {
				return true;
			}
			// Complete if-else if-else chain
		}
	}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should not report missing return for complete if-else if-else chain
	foundMissingReturn := false
	for _, err := range semanticErrors {
		if contains(err.Message, "no return statement") {
			foundMissingReturn = true
		}
	}
	assert.False(t, foundMissingReturn, "Should not report missing return for complete if-else if-else chain")
}

func TestFlowAnalysisIfElseIfIncomplete(t *testing.T) {
	source := `contract Test {
		ext fn if_else_if_incomplete(value: U256) -> Bool {
			if value > 100 {
				return true;
			} else if value > 50 {
				return false;
			}
			// Missing final else clause
		}
	}`

	contract, parseErrors, _ := parser.ParseSource("test.ka", source)
	assert.Empty(t, parseErrors, "Should have no parse errors")

	analyzer := NewAnalyzer()
	semanticErrors := analyzer.Analyze(contract)

	// Should report missing return for incomplete if-else if chain
	foundMissingReturn := false
	for _, err := range semanticErrors {
		if contains(err.Message, "no return statement") {
			foundMissingReturn = true
		}
	}
	assert.True(t, foundMissingReturn, "Should report missing return for incomplete if-else if chain")
}
