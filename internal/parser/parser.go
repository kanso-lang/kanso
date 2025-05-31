package parser

import (
	"fmt"
	"github.com/alecthomas/participle/v2"
	"kanso/grammar"
	"os"
)

var parser = buildParser()

func buildParser() *participle.Parser[grammar.AST] {
	p, err := participle.Build[grammar.AST](
		participle.Lexer(grammar.KansoLexer),
		participle.Elide("Whitespace"),
		participle.UseLookahead(3),
	)
	if err != nil {
		panic(fmt.Errorf("failed to build parser: %w", err))
	}

	return p
}

func ParseFile(path string) (*grammar.AST, error) {
	source, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read file: %w", err)
	}

	return ParseSource(path, string(source))
}

func ParseSource(sourceName string, source string) (*grammar.AST, error) {
	ast, err := parser.ParseString(sourceName, source)
	return ast, err
}
