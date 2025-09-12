// SPDX-License-Identifier: Apache-2.0
package main

import (
	"fmt"
	"github.com/fatih/color"
	"kanso/internal/errors"
	"kanso/internal/parser"
	"kanso/internal/semantic"
	"os"
	"strings"
	"time"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Usage: kanso <file.ka>")
		os.Exit(1)
	}

	startTime := time.Now()
	path := os.Args[1]

	source, err := os.ReadFile(path)
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to read file: %v\n", err)
		os.Exit(1)
	}

	contract, parseErrors, scannerErrors := parser.ParseSource(path, string(source))

	// Create error reporter
	errorReporter := errors.NewErrorReporter(path, string(source))

	// Report scanner errors
	for _, err := range scannerErrors {
		fmt.Print(FormatScanError(path, err, string(source)))
	}

	// Report parser errors
	for _, err := range parseErrors {
		fmt.Print(FormatParseError(path, err, string(source)))
	}

	// Run semantic analysis if parsing succeeded
	hasErrors := len(scannerErrors) > 0 || len(parseErrors) > 0
	if contract != nil {
		analyzer := semantic.NewAnalyzer()
		analyzer.Analyze(contract)

		// Report semantic errors
		semanticErrors := analyzer.GetErrors()
		for _, err := range semanticErrors {
			fmt.Print(errorReporter.FormatError(err))
			hasErrors = true
		}
	}

	// Calculate processing time
	duration := time.Since(startTime)
	formattedDuration := formatDuration(duration)

	// Only print AST and success message if no errors
	if !hasErrors {
		fmt.Println(contract.String())
		color.Green("Successfully processed %s in %s", path, formattedDuration)
	} else {
		color.Red("Compilation failed after %s", formattedDuration)
		os.Exit(1)
	}
}

func formatDuration(d time.Duration) string {
	switch {
	case d >= time.Minute:
		return fmt.Sprintf("%.2fmin", d.Minutes())
	case d >= time.Second:
		return fmt.Sprintf("%.2fs", d.Seconds())
	case d >= time.Millisecond:
		return fmt.Sprintf("%.1fms", float64(d.Nanoseconds())/1000000.0)
	case d >= time.Microsecond:
		return fmt.Sprintf("%.1fμs", float64(d.Nanoseconds())/1000.0)
	default:
		return fmt.Sprintf("%dns", d.Nanoseconds())
	}
}

func FormatScanError(path string, err parser.ScanError, source string) string {
	return formatError(path, err.Message, err.Position, err.Length, source)
}

func FormatParseError(path string, err parser.ParseError, source string) string {
	return formatError(path, err.Message, err.Position, 1, source)
}

func formatError(path, message string, pos parser.Position, length int, source string) string {
	lines := strings.Split(source, "\n")

	var lineContent string
	if pos.Line-1 < len(lines) && pos.Line-1 >= 0 {
		lineContent = lines[pos.Line-1]
	} else {
		lineContent = ""
	}

	// Prepare the underline
	marker := strings.Repeat(" ", max(0, pos.Column-1)) +
		strings.Repeat("^", max(1, length))

	// Color setup
	red := color.New(color.FgRed).SprintFunc()
	bold := color.New(color.Bold).SprintFunc()

	// Compute width for line number column
	lineNumberWidth := len(fmt.Sprintf("%d", pos.Line))
	if lineNumberWidth < 3 {
		lineNumberWidth = 3 // minimum width for visual alignment
	}
	indent := strings.Repeat(" ", lineNumberWidth)

	return fmt.Sprintf(
		"%s: %s\n%s┌─ %s:%d:%d\n%s│\n%3d│%s\n%s│%s\n\n",
		red("error"),
		message,
		indent,
		path, pos.Line, pos.Column,
		indent,
		pos.Line, lineContent,
		indent,
		bold(marker),
	)
}
