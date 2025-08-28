package parser

// regenerate tokentype_string.go with `go generate ./internal/parser`
//
//go:generate stringer -type=TokenType
type TokenType int

const (
	// Special tokens
	ILLEGAL TokenType = iota
	EOF

	// Identifiers + literals
	IDENTIFIER
	NUMBER
	HEX_NUMBER
	STRING

	// Keywords
	FUN
	LET
	IF
	ELSE
	RETURN
	MODULE
	ASSERT
	USE
	STRUCT
	WRITES
	READS
	PUBLIC
	MUT

	// Operators
	PLUS
	INCREMENT
	MINUS
	DECREMENT
	STAR
	STAR_STAR
	SLASH
	BANG
	BANG_EQUAL
	EQUAL
	EQUAL_EQUAL
	LESS
	LESS_EQUAL
	GREATER
	GREATER_EQUAL
	AND
	AMPERSAND
	OR
	PIPE

	// Assignment operators
	PLUS_EQUAL
	MINUS_EQUAL
	STAR_EQUAL
	SLASH_EQUAL
	PERCENT_EQUAL

	// Separators
	COMMA
	DOT
	SEMICOLON
	COLON
	DOUBLE_COLON

	// Brackets
	LEFT_PAREN
	RIGHT_PAREN
	LEFT_BRACE
	RIGHT_BRACE
	LEFT_BRACKET
	RIGHT_BRACKET
	POUND

	// Comments
	COMMENT
	DOC_COMMENT
	BLOCK_COMMENT
)

type Position struct {
	Line   int // 1-based
	Column int // 1-based
	Offset int // 0-based absolute index in input
}
