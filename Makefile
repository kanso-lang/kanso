# Makefile for Kanso compiler + LSP

# Binaries
CLI_BIN = kanso
LSP_BIN = kanso-lsp

CLI_SRC = $(shell find cmd/kanso-cli internal grammar -type f -name '*.go')
LSP_SRC = $(shell find cmd/kanso-lsp internal grammar -type f -name '*.go')

# Module entry points
MODULE_CLI = ./cmd/kanso-cli
MODULE_LSP = ./cmd/kanso-lsp

# Default target: build both
all: $(CLI_BIN) $(LSP_BIN)

# Build the CLI binary (strict mode, no editor tag)
$(CLI_BIN): $(CLI_SRC)
	go build -o $(CLI_BIN) $(MODULE_CLI)

# Build the LSP binary (editor mode with error-tolerant grammar)
$(LSP_BIN): $(LSP_SRC)
	go build -tags editor -o $(LSP_BIN) $(MODULE_LSP)

# Clean all binaries
clean:
	rm -f $(CLI_BIN) $(LSP_BIN)

.PHONY: all run-cli run-lsp clean