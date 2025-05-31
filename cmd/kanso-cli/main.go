// SPDX-License-Identifier: Apache-2.0
package main

import (
	"fmt"
	"github.com/alecthomas/participle/v2"
	"github.com/fatih/color"
	"kanso/internal/parser"
	"os"
	"strings"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Usage: kanso <file.ka>")
		os.Exit(1)
	}

	path := os.Args[1]

	source, err := os.ReadFile(path)
	if err != nil {
		fmt.Errorf("failed to read file: %w", err)
		os.Exit(1)
	}

	ast, err := parser.ParseSource(path, string(source))
	if err != nil {
		reportParseError(string(source), err)
		os.Exit(1)
	}

	fmt.Println(ast.String())

	color.Green("✅ Successfully processed %s", path)
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
