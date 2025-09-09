package parser

var KEYWORDS = map[string]TokenType{
	"fn":       FN,
	"let":      LET,
	"if":       IF,
	"else":     ELSE,
	"return":   RETURN,
	"contract": CONTRACT,
	"require":  REQUIRE,
	"use":      USE,
	"struct":   STRUCT,
	"writes":   WRITES,
	"reads":    READS,
	"ext":      EXT,
	"mut":      MUT,
}
