package lsp

import (
	protocol "github.com/tliron/glsp/protocol_3_16"
	"kanso/internal/parser"
)

// ConvertParseErrors transforms parser errors into LSP diagnostics for IDE display.
// These provide immediate feedback about syntax issues like missing brackets,
// semicolons, commas in struct declarations, and other parsing problems.
func ConvertParseErrors(parseErrors []parser.ParseError) []protocol.Diagnostic {
	var diagnostics []protocol.Diagnostic

	for _, parseErr := range parseErrors {
		diagnostic := protocol.Diagnostic{
			Range: protocol.Range{
				Start: protocol.Position{
					Line:      uint32(parseErr.Position.Line - 1),   // Convert to 0-based indexing
					Character: uint32(parseErr.Position.Column - 1), // Convert to 0-based indexing
				},
				End: protocol.Position{
					Line:      uint32(parseErr.Position.Line - 1),
					Character: uint32(parseErr.Position.Column + 5), // Rough span for visibility
				},
			},
			Severity: ptrSeverity(protocol.DiagnosticSeverityError),
			Source:   ptrString("kanso-parser"),
			Message:  parseErr.Message,
		}
		diagnostics = append(diagnostics, diagnostic)
	}

	return diagnostics
}

// ConvertScanErrors transforms scanner errors into LSP diagnostics for IDE display.
// These handle tokenization issues like invalid characters, unterminated strings, etc.
func ConvertScanErrors(scanErrors []parser.ScanError) []protocol.Diagnostic {
	var diagnostics []protocol.Diagnostic

	for _, scanErr := range scanErrors {
		// Use the Length field if available, otherwise default span
		endChar := uint32(scanErr.Position.Column - 1 + scanErr.Length)
		if scanErr.Length == 0 {
			endChar = uint32(scanErr.Position.Column + 3) // Default small span
		}

		diagnostic := protocol.Diagnostic{
			Range: protocol.Range{
				Start: protocol.Position{
					Line:      uint32(scanErr.Position.Line - 1),   // Convert to 0-based indexing
					Character: uint32(scanErr.Position.Column - 1), // Convert to 0-based indexing
				},
				End: protocol.Position{
					Line:      uint32(scanErr.Position.Line - 1),
					Character: endChar,
				},
			},
			Severity: ptrSeverity(protocol.DiagnosticSeverityError),
			Source:   ptrString("kanso-scanner"),
			Message:  scanErr.Message,
		}
		diagnostics = append(diagnostics, diagnostic)
	}

	return diagnostics
}

// Legacy function kept for compatibility - delegates to the new functions
func ConvertParseError(err error) []protocol.Diagnostic {
	// This function is kept for compatibility but should not be used
	// All calls should use ConvertParseErrors and ConvertScanErrors instead
	return []protocol.Diagnostic{}
}

//func CollectDiagnostics(contract *ast.Contract) []protocol.Diagnostic {
//	var diagnostics []protocol.Diagnostic
//
//	if ast == nil {
//		return diagnostics
//	}
//
//	for _, se := range ast.SourceElements {
//		if se.Module != nil {
//			diagnostics = append(diagnostics, walkModuleForErrors(se.Module)...)
//		}
//	}
//
//	return diagnostics
//}
//
//func walkModuleForErrors(m *grammar.Module) []protocol.Diagnostic {
//	var diagnostics []protocol.Diagnostic
//
//	for _, f := range m.Functions {
//		diagnostics = append(diagnostics, walkFunctionBlockForErrors(f.Body)...)
//	}
//
//	return diagnostics
//}
//
//func walkFunctionBlockForErrors(fb *grammar.FunctionBlock) []protocol.Diagnostic {
//	var diagnostics []protocol.Diagnostic
//
//	if fb == nil {
//		return diagnostics
//	}
//
//	for _, stmt := range fb.Statements {
//		if stmt.Error != nil {
//			diagnostics = append(diagnostics, errorNodeToDiagnostic(stmt.Error)...)
//		}
//	}
//
//	return diagnostics
//}
//
//func errorNodeToDiagnostic(errNode *grammar.ErrorNode) []protocol.Diagnostic {
//	var diagnostics []protocol.Diagnostic
//
//	for _, unexpected := range errNode.Unexpected {
//		diag := protocol.Diagnostic{
//			Range: protocol.Range{
//				Start: protocol.Position{Line: uint32(errNode.Pos.Line - 1), Character: uint32(errNode.Pos.Column - 1)},
//				End:   protocol.Position{Line: uint32(errNode.Pos.Line - 1), Character: uint32(errNode.Pos.Column)},
//			},
//			Severity: ptrSeverity(protocol.DiagnosticSeverityError),
//			Source:   stringPtr("kanso-parser"),
//			Message:  fmt.Sprintf("Unexpected token: %s", unexpected),
//		}
//		diagnostics = append(diagnostics, diag)
//	}
//
//	return diagnostics
//}
//
//func stringPtr(s string) *string {
//	return &s
//}
//
//func ptrSeverity(s protocol.DiagnosticSeverity) *protocol.DiagnosticSeverity {
//	return &s
//}
//
//func ptrString(s string) *string {
//	return &s
//}
