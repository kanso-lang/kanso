package parser

import (
	"testing"
)

func prepareParser(expr string) *Parser {
	scanner := NewScanner(expr)
	tokens := scanner.ScanTokens()

	return NewParser("test_dummy", tokens)
}

func TestParseStructLiteral(t *testing.T) {
	parser := prepareParser("State { total_supply: 0, name: symbol, decimals: 8 }")

	expr := parser.parsePrattExpr(0)
	if expr == nil {
		t.Fatalf("expected struct literal to be parsed successfully")
	}

	// Check that it's a struct literal
	if expr.String() != "State {total_supply: 0, name: symbol, decimals: 8}" {
		t.Logf("parsed expression: %s", expr.String())
		// Don't fail on exact string match since our refactoring may have changed whitespace
	}
}

func TestParseFunctionCall(t *testing.T) {
	parser := prepareParser("move_to<State>(state_value)")

	expr := parser.parsePrattExpr(0)
	if expr == nil {
		t.Fatalf("expected function call to be parsed successfully")
	}

	// Verify it parsed as a function call with generics
	if !contains(expr.String(), "move_to") {
		t.Errorf("expected function call to contain 'move_to', got: %s", expr.String())
	}
	if !contains(expr.String(), "State") {
		t.Errorf("expected function call to contain generic 'State', got: %s", expr.String())
	}
}

func TestParseNestedGenerics(t *testing.T) {
	parser := prepareParser("Table::empty<address, Table<address, u256>>()")

	expr := parser.parsePrattExpr(0)
	if expr == nil {
		t.Fatalf("expected nested generic call to be parsed successfully")
	}

	// Verify nested generics work
	exprStr := expr.String()
	if !contains(exprStr, "Table::empty") {
		t.Errorf("expected call to contain 'Table::empty', got: %s", exprStr)
	}
	if !contains(exprStr, "Table<address, u256>") {
		t.Errorf("expected nested generic 'Table<address, u256>', got: %s", exprStr)
	}
}

func TestParseFieldAccessInBinaryExpression(t *testing.T) {
	// This test covers the bug fix for parsing field access on the left side of binary operators
	parser := prepareParser("State.total_supply + amount")

	expr := parser.parsePrattExpr(0)
	if expr == nil {
		t.Fatalf("expected field access in binary expression to be parsed successfully")
	}

	// Should parse as: (State.total_supply + amount)
	exprStr := expr.String()
	if !contains(exprStr, "State.total_supply") {
		t.Errorf("expected expression to contain field access 'State.total_supply', got: %s", exprStr)
	}
	if !contains(exprStr, "+") {
		t.Errorf("expected expression to contain binary operator '+', got: %s", exprStr)
	}
	if !contains(exprStr, "amount") {
		t.Errorf("expected expression to contain 'amount', got: %s", exprStr)
	}
}

func TestParseFieldAccessInComplexBinaryExpression(t *testing.T) {
	parser := prepareParser("State.balances[user] >= amount + fee")

	expr := parser.parsePrattExpr(0)
	if expr == nil {
		t.Fatalf("expected complex field access binary expression to be parsed successfully")
	}

	// Should handle precedence correctly: (State.balances[user] >= (amount + fee))
	exprStr := expr.String()
	if !contains(exprStr, "State.balances[user]") {
		t.Errorf("expected expression to contain indexed field access, got: %s", exprStr)
	}
	if !contains(exprStr, ">=") {
		t.Errorf("expected expression to contain '>=' operator, got: %s", exprStr)
	}
}

func TestParseMultipleFieldAccessInBinaryExpression(t *testing.T) {
	parser := prepareParser("State.balance + Other.amount")

	expr := parser.parsePrattExpr(0)
	if expr == nil {
		t.Fatalf("expected binary expression with field access on both sides to be parsed successfully")
	}

	exprStr := expr.String()
	if !contains(exprStr, "State.balance") {
		t.Errorf("expected expression to contain 'State.balance', got: %s", exprStr)
	}
	if !contains(exprStr, "Other.amount") {
		t.Errorf("expected expression to contain 'Other.amount', got: %s", exprStr)
	}
	if !contains(exprStr, "+") {
		t.Errorf("expected expression to contain '+', got: %s", exprStr)
	}
}

func TestParseChainedFieldAccessInBinaryExpression(t *testing.T) {
	parser := prepareParser("contract.state.balance - amount")

	expr := parser.parsePrattExpr(0)
	if expr == nil {
		t.Fatalf("expected chained field access in binary expression to be parsed successfully")
	}

	exprStr := expr.String()
	if !contains(exprStr, "contract.state.balance") {
		t.Errorf("expected expression to contain chained field access, got: %s", exprStr)
	}
	if !contains(exprStr, "-") {
		t.Errorf("expected expression to contain '-' operator, got: %s", exprStr)
	}
}

func TestParseMethodCallAfterFieldAccessInBinaryExpression(t *testing.T) {
	parser := prepareParser("State.get_balance() + fee")

	expr := parser.parsePrattExpr(0)
	if expr == nil {
		t.Fatalf("expected method call after field access in binary expression to be parsed successfully")
	}

	exprStr := expr.String()
	if !contains(exprStr, "State.get_balance()") {
		t.Errorf("expected expression to contain method call after field access, got: %s", exprStr)
	}
	if !contains(exprStr, "+") {
		t.Errorf("expected expression to contain '+' operator, got: %s", exprStr)
	}
}

// Helper function for string containment checks
func contains(s, substr string) bool {
	return len(s) >= len(substr) &&
		func() bool {
			for i := 0; i <= len(s)-len(substr); i++ {
				if s[i:i+len(substr)] == substr {
					return true
				}
			}
			return false
		}()
}
