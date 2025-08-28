package ast

type AssignType int

// regenerate tokentype_string.go with `go generate ./internal/ast`
//
//go:generate stringer -type=AssignType
const (
	// Special / error
	ILLEGAL_ASSIGN AssignType = iota
	ASSIGN
	PLUS_ASSIGN
	MINUS_ASSIGN
	STAR_ASSIGN
	SLASH_ASSIGN
	PERCENT_ASSIGN
)
