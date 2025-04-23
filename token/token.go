// Package token SPDX-License-Identifier: Apache-2.0
package token

type TokenType string

type Token struct {
	Type    TokenType
	Literal string
}

const (
	ILLEGAL = "ILLEGAL"
	EOF     = "EOF"

	// Identifiers + literals
	IDENT = "IDENT" // add, foobar, x, y ...
	INT   = "INT"   // 1234567890

	// Operators
	ASSIGN   = "="
	PLUS     = "+"
	MINUS    = "-"
	BANG     = "!"
	ASTERISK = "*"
	SLASH    = "/"

	LT = "<"
	GT = ">"

	AMPERSAND = "&"

	EQ     = "=="
	NOT_EQ = "!="

	// Delimiters
	COMMA     = ","
	SEMICOLON = ";"
	COLON     = ":"
	NAMESPACE = "::"

	LPAREN = "("
	RPAREN = ")"
	LBRACE = "{"
	RBRACE = "}"

	LATTR = " #["
	RATTR = "]"

	DOC_COMMENT = "DOC_COMMENT"
	COMMENT     = "COMMENT"

	// Keywords
	FUNCTION = "FUNCTION"
	LET      = "LET"
	TRUE     = "TRUE"
	FALSE    = "FALSE"
	IF       = "IF"
	ELSE     = "ELSE"
	RETURN   = "RETURN"
	PUBLIC   = "PUBLIC"
	ACQUIRES = "ACQUIRES"
	MODULE   = "MODULE"
	USE      = "USE"
	STRUCT   = "STRUCT"
	HAS      = "HAS"
)

var keywords = map[string]TokenType{
	"fun":      FUNCTION,
	"public":   PUBLIC,
	"let":      LET,
	"true":     TRUE,
	"false":    FALSE,
	"if":       IF,
	"else":     ELSE,
	"return":   RETURN,
	"acquires": ACQUIRES,
	"module":   MODULE,
	"use":      USE,
	"struct":   STRUCT,
	"has":      HAS,
}

func LookupIdent(ident string) TokenType {
	if tok, ok := keywords[ident]; ok {
		return tok
	}
	return IDENT
}
