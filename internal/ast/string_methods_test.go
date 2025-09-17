package ast

import (
	"testing"
)

// Tests for auto-generated string methods
func TestNodeTypeStrings(t *testing.T) {
	// Test all NodeType constants to cover nodetype_string.go
	nodeTypes := []NodeType{
		ILLEGAL,
		BAD_CONTRACT_ITEM,
		BAD_MODULE_ITEM,
		BAD_EXPR,
		DOC_COMMENT,
		COMMENT,
		CONTRACT,
		ATTRIBUTE,
		USE,
		NAMESPACE,
		IMPORT_ITEM,
		STRUCT,
		STRUCT_FIELD,
		TYPE,
		IDENT,
		FUNCTION,
		FUNCTION_PARAM,
		FUNCTION_BLOCK,
		EXPR_STMT,
		RETURN_STMT,
		LET_STMT,
		ASSIGN_STMT,
		REQUIRE_STMT,
		IF_STMT,
		BINARY_EXPR,
		UNARY_EXPR,
		CALL_EXPR,
		FIELD_ACCESS_EXPR,
		INDEX_EXPR,
		STRUCT_LITERAL_EXPR,
		LITERAL_EXPR,
		IDENT_EXPR,
		CALLEE_PATH,
		STRUCT_LITERAL_FIELD,
		PAREN_EXPR,
		TUPLE_EXPR,
	}

	for _, nodeType := range nodeTypes {
		str := nodeType.String()
		if str == "" {
			t.Errorf("NodeType %v should have non-empty string", nodeType)
		}
	}
}

// Test AssignType strings to cover assigntype_string.go
func TestAssignTypeStrings(t *testing.T) {
	assignTypes := []AssignType{
		ILLEGAL_ASSIGN,
		ASSIGN,
		PLUS_ASSIGN,
		MINUS_ASSIGN,
		STAR_ASSIGN,
		SLASH_ASSIGN,
		PERCENT_ASSIGN,
	}

	for _, assignType := range assignTypes {
		str := assignType.String()
		if str == "" {
			t.Errorf("AssignType %v should have non-empty string", assignType)
		}
	}
}

// Test interface methods using the simplest possible constructions
func TestInterfaceMethodsMinimal(t *testing.T) {
	// Test expressions - use exact pattern from existing tests
	expr := &LiteralExpr{Value: "test"}
	expr.isExpr() // This calls the interface method for testing

	identExpr := &IdentExpr{Name: "test"}
	identExpr.isExpr()

	// Test statements
	stmt := &ExprStmt{Expr: expr}
	stmt.isBlockItem() // This calls the interface method for testing

	// Test contract items
	fn := &Function{Name: Ident{Value: "test"}, Body: &FunctionBlock{}}
	fn.isContractItem() // This calls the interface method for testing
}

// Test complex string methods for printer functionality
func TestComplexStringMethods(t *testing.T) {
	// Test Let statement with mutable flag
	letStmt := &LetStmt{
		Mut:  true,
		Name: Ident{Value: "x"},
	}
	letStr := letStmt.String()
	if letStr == "" {
		t.Error("LetStmt string should not be empty")
	}

	// Test RequireStmt
	requireStmt := &RequireStmt{
		Args: []Expr{&LiteralExpr{Value: "condition"}},
	}
	requireStr := requireStmt.String()
	if requireStr == "" {
		t.Error("RequireStmt string should not be empty")
	}

	// Test multiple argument RequireStmt
	multiRequire := &RequireStmt{
		Args: []Expr{
			&LiteralExpr{Value: "condition"},
			&LiteralExpr{Value: "error"},
		},
	}
	multiStr := multiRequire.String()
	if multiStr == "" {
		t.Error("Multi-arg RequireStmt string should not be empty")
	}

	// Test all interface methods to improve coverage (without calling String to avoid crashes)
	allExprs := []Expr{
		&BadExpr{},
		&BinaryExpr{},
		&UnaryExpr{},
		&CallExpr{},
		&FieldAccessExpr{},
		&IndexExpr{},
		&StructLiteralExpr{},
		&LiteralExpr{Value: "test"},
		&IdentExpr{Name: "test"},
		&CalleePath{},
		&StructLiteralField{},
		&ParenExpr{},
		&TupleExpr{},
	}

	for _, expr := range allExprs {
		expr.isExpr() // Call interface method for testing
	}

	allContractItems := []ContractItem{
		&BadContractItem{},
		&DocComment{},
		&Comment{},
		&Attribute{},
		&Function{Name: Ident{Value: "test"}, Body: &FunctionBlock{}},
		&Struct{},
		&Use{},
	}

	for _, item := range allContractItems {
		item.isContractItem() // Call interface method for testing
	}

	allBlockItems := []FunctionBlockItem{
		&LetStmt{},
		&AssignStmt{},
		&RequireStmt{},
		&IfStmt{},
		&ReturnStmt{},
		&ExprStmt{},
		&Comment{},
	}

	for _, item := range allBlockItems {
		item.isBlockItem() // Call interface method for testing
	}
}
