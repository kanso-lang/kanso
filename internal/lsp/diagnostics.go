package lsp

import (
	protocol "github.com/tliron/glsp/protocol_3_16"
)

// ConvertParseError turns a participle parse error into an LSP diagnostic.
func ConvertParseError(err error) []protocol.Diagnostic {
	if err == nil {
		return nil
	}

	var diagnostics []protocol.Diagnostic

	// Check if it's a participle.ParseError
	//if perr, ok := err.(participle.Error); ok {
	//	diagnostics = append(diagnostics, protocol.Diagnostic{
	//		Range: protocol.Range{
	//			Start: protocol.Position{
	//				Line:      uint32(perr.Position().Line - 1),
	//				Character: uint32(perr.Position().Column - 1),
	//			},
	//			End: protocol.Position{
	//				Line:      uint32(perr.Position().Line - 1),
	//				Character: uint32(perr.Position().Column),
	//			},
	//		},
	//		Severity: ptrSeverity(protocol.DiagnosticSeverityError),
	//		Source:   ptrString("kanso-parser"),
	//		Message:  perr.Message(),
	//	})
	//} else {
	//	// Generic fallback diagnostic
	//	diagnostics = append(diagnostics, protocol.Diagnostic{
	//		Range: protocol.Range{
	//			Start: protocol.Position{Line: 0, Character: 0},
	//			End:   protocol.Position{Line: 0, Character: 1},
	//		},
	//		Severity: ptrSeverity(protocol.DiagnosticSeverityError),
	//		Source:   ptrString("kanso-parser"),
	//		Message:  fmt.Sprintf("Parse error: %v", err),
	//	})
	//}

	return diagnostics
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
