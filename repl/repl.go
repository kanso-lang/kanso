// Package repl SPDX-License-Identifier: Apache-2.0
package repl

import (
	"bufio"
	"fmt"
	"io"
	"kanso-lang/lexer"
	"kanso-lang/parser"
)

const PROMPT = ">> "

func Start(in io.Reader) {
	scanner := bufio.NewScanner(in)

	for {
		fmt.Print(PROMPT)
		scanned := scanner.Scan()
		if !scanned {
			continue
		}

		line := scanner.Text()
		l := lexer.New(line)
		p := parser.New(l)

		program := p.ParseProgram()
		if program == nil {
			fmt.Println("ParseProgram() returned nil")
		}

		fmt.Printf("AST:\n%s\n", program.String())
	}
}
