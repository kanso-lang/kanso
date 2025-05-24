// SPDX-License-Identifier: Apache-2.0
package main

import (
	"fmt"
	"github.com/alecthomas/participle/v2"
	"github.com/fatih/color"
	"kanso/grammar"
	"os"
	"strings"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Usage: kanso <file.kanso>")
		os.Exit(1)
	}

	path := os.Args[1]
	source, err := os.ReadFile(path)
	if err != nil {
		color.Red("Failed to read file: %s", err)
		os.Exit(1)
	}

	parser, err := participle.Build[grammar.Program](
		participle.Lexer(grammar.KansoLexer),
		participle.Elide("Whitespace"),
		// Use lookahead to avoid ambiguity between attributes for structs and functions
		participle.UseLookahead(3),
	)
	if err != nil {
		color.Red("Parser build failed: %s", err)
		os.Exit(1)
	}

	program, err := parser.ParseString(path, string(source))
	if err != nil {
		reportParseError(string(source), err)
		os.Exit(1)
	}

	fmt.Println("Parsed program:")
	fmt.Print(program.String())

	color.Green("✅ Successfully parsed %s", path)
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
