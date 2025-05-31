// SPDX-License-Identifier: Apache-2.0
package main

import (
	"kanso/internal/lsp"
	"log"
	"os"

	"github.com/tliron/commonlog"
	protocol "github.com/tliron/glsp/protocol_3_16"
	"github.com/tliron/glsp/server"
)

const lsName = "kanso" // Name identifier for the language server

var (
	version = "0.0.1"        // Server version
	handler protocol.Handler // Protocol handler instance (wired up below)
)

// Define the list of supported semantic token types
// This should align with what your language server reports and uses
var semanticTokenTypes = []string{
	"namespace",
	"type",
	"typeParameter",
	"function",
	"variable",
	"parameter",
	"property",
	"keyword",
	"number",
	"operator",
}

// Define the list of semantic token modifiers (extra tags)
var semanticTokenModifiers = []string{
	"declaration",
	"definition",
	"readonly",
	"static",
	"deprecated",
	"abstract",
}

func main() {
	// Configure debug logging (1 = debug level, nil = default logger)
	commonlog.Configure(1, nil)

	// Create a new instance of the KansoHandler (your language-specific handler)
	kansoHandler := lsp.NewKansoHandler()

	// Wire up the handler with specific LSP method implementations
	handler = protocol.Handler{
		Initialize:                     kansoHandler.Initialize,
		Initialized:                    kansoHandler.Initialized,
		Shutdown:                       kansoHandler.Shutdown,
		SetTrace:                       kansoHandler.SetTrace,
		TextDocumentDidOpen:            kansoHandler.TextDocumentDidOpen,
		TextDocumentDidClose:           kansoHandler.TextDocumentDidClose,
		TextDocumentDidChange:          kansoHandler.TextDocumentDidChange,
		TextDocumentCompletion:         kansoHandler.TextDocumentCompletion,
		TextDocumentSemanticTokensFull: kansoHandler.TextDocumentSemanticTokensFull,
	}

	// Create a new GLSP (Go Language Server Protocol) server instance
	// Parameters:
	// - handler: the protocol handler struct
	// - name: the language server name (shown to clients)
	// - debug: whether to enable internal GLSP debug logs
	s := server.NewServer(&handler, lsName, false)

	log.Println("Starting Kanso LSP server...")

	// Start the server over standard input/output (used by most editors for LSP)
	// This lets the editor communicate with the language server process
	err := s.RunStdio()
	if err != nil {
		log.Println("Error starting Kanso LSP server:", err)
		os.Exit(1)

	}
}
