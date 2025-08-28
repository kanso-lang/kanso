package ast

type NodeType int

// regenerate tokentype_string.go with `go generate ./internal/ast`
//
//go:generate stringer -type=NodeType
const (
	// Special / error
	ILLEGAL NodeType = iota
	BAD_CONTRACT_ITEM
	BAD_MODULE_ITEM
	BAD_EXPR

	// Comments
	DOC_COMMENT
	COMMENT

	// High-level constructs
	MODULE

	// Attributes
	ATTRIBUTE

	// Imports / uses
	USE
	NAMESPACE
	IMPORT_ITEM

	// Structs
	STRUCT
	STRUCT_FIELD

	// Types
	TYPE
	REF_TYPE
	IDENT

	// Functions
	FUNCTION
	FUNCTION_PARAM

	// Statements
	FUNCTION_BLOCK
	EXPR_STMT
	RETURN_STMT
	LET_STMT
	ASSIGN_STMT
	ASSERT_STMT

	// Expressions
	BINARY_EXPR
	UNARY_EXPR
	CALL_EXPR
	FIELD_ACCESS_EXPR
	STRUCT_LITERAL_EXPR
	LITERAL_EXPR
	IDENT_EXPR
	CALLEE_PATH
	STRUCT_LITERAL_FIELD
	PAREN_EXPR
)
