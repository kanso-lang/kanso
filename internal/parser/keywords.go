package parser

var KEYWORDS = map[string]TokenType{
	"fun":    FUN,
	"let":    LET,
	"if":     IF,
	"else":   ELSE,
	"return": RETURN,
	"module": MODULE,
	"assert": ASSERT,
	"use":    USE,
	"struct": STRUCT,
	"writes": WRITES,
	"reads":  READS,
	"public": PUBLIC,
	"mut":    MUT,
}
