// SPDX-License-Identifier: Apache-2.0
package main

import (
	"fmt"
	"github.com/fatih/color"
	"kanso/grammar"
	"os"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Usage: kanso <file.ka>")
		os.Exit(1)
	}

	path := os.Args[1]

	program, err := grammar.ParseFile(path)
	if err != nil {
		os.Exit(1)
	}

	fmt.Println("Parsed program:")
	fmt.Print(program.String())

	color.Green("✅ Successfully parsed %s", path)
}
