package grammar

import (
	"fmt"
	"github.com/alecthomas/participle/v2"
	"github.com/fatih/color"
	"os"
	"strings"
)

func ParseFile(path string) (*Program, error) {
	source, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read file: %w", err)
	}

	parser, err := participle.Build[Program](
		participle.Lexer(KansoLexer),
		participle.Elide("Whitespace"),
		participle.UseLookahead(3),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to build parser: %w", err)
	}

	program, err := parser.ParseString(path, string(source))
	if err != nil {
		reportParseError(string(source), err)
		return nil, err
	}
	return program, nil
}

// reportParseError prints a friendly caret-style parse error message.
func reportParseError(src string, err error) {
	pe, ok := err.(participle.Error)
	if !ok {
		color.Red("Unexpected error: %s", err)
		return
	}

	pos := pe.Position()
	lines := strings.Split(src, "\n")
	if pos.Line <= 0 || pos.Line > len(lines) {
		color.Red("Syntax error at unknown location: %s", err)
		return
	}

	line := lines[pos.Line-1]
	caret := strings.Repeat(" ", pos.Column-1) + "^"

	color.Red("❌ Syntax error in %s at line %d, column %d:", pos.Filename, pos.Line, pos.Column)
	fmt.Println(line)
	color.HiRed(caret)
	fmt.Printf("→ %s\n", pe.Message())
}
