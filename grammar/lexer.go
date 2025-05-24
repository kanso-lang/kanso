package grammar

import (
	"github.com/alecthomas/participle/v2/lexer"
)

var KansoLexer = lexer.MustStateful(lexer.Rules{
	"Root": {
		// Comments
		{"DocComment", `///[^\n]*`, nil},
		// Comments
		{"Comment", `//[^\n]*`, nil},

		// Keywords and Identifiers (order matters)
		{"Ident", `[a-zA-Z_][a-zA-Z0-9_]*`, nil},

		// Integer literals
		{"Integer", `0x[0-9a-fA-F]+|[0-9]+`, nil},

		// Operators
		{"Operator", `(\|\||&&|==|!=|<=|>=|\+=|-=|\*=|/=|%=|=|[-+*/%&|<>])`, nil},

		// Punctuation (must come after operators)
		{"Punctuation", `[{}[\]#:,;<>()<>.!*-]`, nil},

		// Special tokens
		{"Symbol", `[!]`, nil}, // For assert!, unary not, etc.

		// Whitespace
		{"Whitespace", `[ \t\r\n]+`, nil},
	},
})
