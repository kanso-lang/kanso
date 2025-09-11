package errors

import (
	"fmt"
	"strings"

	"github.com/fatih/color"
	"kanso/internal/ast"
)

// ErrorLevel represents the severity of an error
type ErrorLevel string

const (
	Error   ErrorLevel = "error"
	Warning ErrorLevel = "warning"
	Note    ErrorLevel = "note"
	Help    ErrorLevel = "help"
)

// CompilerError represents a structured error with suggestions and context
type CompilerError struct {
	Level       ErrorLevel
	Code        string       // Error code like E0001
	Message     string       // Primary error message
	Position    ast.Position // Location in source
	Length      int          // Length of the problematic region
	Suggestions []Suggestion // Suggested fixes
	Notes       []string     // Additional context notes
	HelpText    string       // Help text for the error
}

// Suggestion represents a suggested fix
type Suggestion struct {
	Message     string       // Description of the suggestion
	Replacement string       // Suggested replacement text (optional)
	Position    ast.Position // Position to apply the fix (optional)
	Length      int          // Length of text to replace (optional)
}

// ErrorReporter handles consistent error formatting and suggestions
type ErrorReporter struct {
	filename string
	source   string
	lines    []string
}

// NewErrorReporter creates a new error reporter for a file
func NewErrorReporter(filename, source string) *ErrorReporter {
	return &ErrorReporter{
		filename: filename,
		source:   source,
		lines:    strings.Split(source, "\n"),
	}
}

// FormatError formats a compiler error with Rust-like styling and suggestions
func (er *ErrorReporter) FormatError(err CompilerError) string {
	var result strings.Builder

	// Color setup
	levelColor := er.getLevelColor(err.Level)
	bold := color.New(color.Bold).SprintFunc()
	dim := color.New(color.Faint).SprintFunc()

	// Header: error[E0001]: message
	if err.Code != "" {
		result.WriteString(fmt.Sprintf("%s[%s]: %s\n",
			levelColor(string(err.Level)), err.Code, err.Message))
	} else {
		result.WriteString(fmt.Sprintf("%s: %s\n",
			levelColor(string(err.Level)), err.Message))
	}

	// Location line: --> filename:line:column
	lineNumberWidth := er.getLineNumberWidth(err.Position.Line)
	indent := strings.Repeat(" ", lineNumberWidth)

	result.WriteString(fmt.Sprintf("%s %s %s:%d:%d\n",
		indent, dim("-->"), er.filename, err.Position.Line, err.Position.Column))

	// Separator line
	result.WriteString(fmt.Sprintf("%s %s\n", indent, dim("│")))

	// Context lines (show line before if available)
	if err.Position.Line > 1 && err.Position.Line-1 < len(er.lines) {
		result.WriteString(fmt.Sprintf("%s %s %s\n",
			dim(fmt.Sprintf("%*d", lineNumberWidth, err.Position.Line-1)),
			dim("│"),
			er.lines[err.Position.Line-2]))
	}

	// Main error line
	if err.Position.Line <= len(er.lines) && err.Position.Line > 0 {
		lineContent := er.lines[err.Position.Line-1]
		result.WriteString(fmt.Sprintf("%s %s %s\n",
			bold(fmt.Sprintf("%*d", lineNumberWidth, err.Position.Line)),
			dim("│"),
			lineContent))

		// Error marker line
		marker := er.createMarker(err.Position.Column, err.Length, err.Level)
		result.WriteString(fmt.Sprintf("%s %s %s\n",
			indent, dim("│"), marker))
	}

	// Context line after if available
	if err.Position.Line < len(er.lines) {
		result.WriteString(fmt.Sprintf("%s %s %s\n",
			dim(fmt.Sprintf("%*d", lineNumberWidth, err.Position.Line+1)),
			dim("│"),
			er.lines[err.Position.Line]))
	}

	// Add suggestions
	if len(err.Suggestions) > 0 {
		result.WriteString(fmt.Sprintf("%s %s\n", indent, dim("│")))
		for i, suggestion := range err.Suggestions {
			suggestionColor := color.New(color.FgCyan).SprintFunc()

			if i == 0 {
				result.WriteString(fmt.Sprintf("%s %s %s: %s\n",
					indent, suggestionColor("help"), suggestionColor("try"), suggestion.Message))
			} else {
				result.WriteString(fmt.Sprintf("%s %s %s\n",
					indent, suggestionColor("    "), suggestion.Message))
			}

			// If suggestion has replacement text, show it
			if suggestion.Replacement != "" {
				result.WriteString(fmt.Sprintf("%s %s\n", indent, dim("│")))
				replacement := strings.ReplaceAll(suggestion.Replacement, "\n", fmt.Sprintf("\n%s %s ", indent, dim("│")))
				result.WriteString(fmt.Sprintf("%s %s %s\n",
					indent, suggestionColor("│"), suggestionColor(replacement)))
			}
		}
	}

	// Add notes
	for _, note := range err.Notes {
		noteColor := color.New(color.FgBlue).SprintFunc()
		result.WriteString(fmt.Sprintf("%s %s %s %s\n",
			indent, dim("│"), noteColor("note:"), note))
	}

	// Add help text
	if err.HelpText != "" {
		helpColor := color.New(color.FgGreen).SprintFunc()
		result.WriteString(fmt.Sprintf("%s %s %s %s\n",
			indent, dim("│"), helpColor("help:"), err.HelpText))
	}

	result.WriteString("\n")
	return result.String()
}

// getLevelColor returns the appropriate color function for an error level
func (er *ErrorReporter) getLevelColor(level ErrorLevel) func(...interface{}) string {
	switch level {
	case Error:
		return color.New(color.FgRed, color.Bold).SprintFunc()
	case Warning:
		return color.New(color.FgYellow, color.Bold).SprintFunc()
	case Note:
		return color.New(color.FgBlue, color.Bold).SprintFunc()
	case Help:
		return color.New(color.FgGreen, color.Bold).SprintFunc()
	default:
		return color.New(color.FgRed, color.Bold).SprintFunc()
	}
}

// createMarker creates the underline marker for errors
func (er *ErrorReporter) createMarker(column, length int, level ErrorLevel) string {
	if length <= 0 {
		length = 1
	}

	spaces := strings.Repeat(" ", max(0, column-1))

	var markerChar string
	var markerColor func(...interface{}) string

	switch level {
	case Error:
		markerChar = "^"
		markerColor = color.New(color.FgRed, color.Bold).SprintFunc()
	case Warning:
		markerChar = "^"
		markerColor = color.New(color.FgYellow, color.Bold).SprintFunc()
	default:
		markerChar = "^"
		markerColor = color.New(color.FgRed, color.Bold).SprintFunc()
	}

	marker := strings.Repeat(markerChar, length)
	return spaces + markerColor(marker)
}

// getLineNumberWidth calculates the width needed for line numbers
func (er *ErrorReporter) getLineNumberWidth(line int) int {
	width := len(fmt.Sprintf("%d", line))
	if width < 3 {
		width = 3 // minimum width for visual alignment
	}
	return width
}

// max returns the maximum of two integers
func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
