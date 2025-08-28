package lsp_test

import (
	"fmt"
	"path/filepath"
	"testing"

	"github.com/stretchr/testify/require"
	"github.com/tliron/glsp"
	protocol "github.com/tliron/glsp/protocol_3_16"

	"kanso/internal/lsp"
)

func TestTextDocumentSemanticTokensFull(t *testing.T) {
	handler := lsp.NewKansoHandler()

	absPath, err := filepath.Abs(filepath.Join("../../examples", "erc20.ka"))
	require.NoError(t, err, "Failed to get absolute path")

	uri := "file://" + filepath.ToSlash(absPath)

	ctx := &glsp.Context{}
	params := &protocol.SemanticTokensParams{
		TextDocument: protocol.TextDocumentIdentifier{
			URI: uri,
		},
	}

	tokens, err := handler.TextDocumentSemanticTokensFull(ctx, params)
	require.NoError(t, err, "TextDocumentSemanticTokensFull returned error")
	require.NotNil(t, tokens, "Returned tokens should not be nil")
	require.NotEmpty(t, tokens.Data, "Returned token data should not be empty")

	decoded, err := decodeSemanticTokens(tokens.Data)
	require.NoError(t, err, "Failed to decode semantic tokens")
	require.NotEmpty(t, decoded, "No semantic tokens decoded")

	// Basic sanity checks - we should have semantic tokens for key language elements
	tokenTypes := make(map[string]int)
	for _, token := range decoded {
		tokenTypes[token.Type]++
	}

	// Verify we have tokens for important language constructs
	require.Greater(t, tokenTypes["modifier"], 0, "Should have modifier tokens for attributes like #[contract]")
	require.Greater(t, tokenTypes["namespace"], 0, "Should have namespace tokens for module names")
	require.Greater(t, tokenTypes["type"], 0, "Should have type tokens for struct names")
	require.Greater(t, tokenTypes["function"], 0, "Should have function tokens for function names")
	require.Greater(t, tokenTypes["property"], 0, "Should have property tokens for struct fields")

	t.Logf("Generated %d semantic tokens with types: %v", len(decoded), tokenTypes)
}

type DecodedToken struct {
	Index     int
	Line      uint32
	Char      uint32
	Length    uint32
	Type      string
	Modifiers []string
}

func decodeSemanticTokens(raw []uint32) ([]DecodedToken, error) {
	if len(raw)%5 != 0 {
		return nil, fmt.Errorf("raw token data length %d is not a multiple of 5", len(raw))
	}

	var (
		decoded []DecodedToken
		line    uint32
		char    uint32
	)

	for i := 0; i < len(raw); i += 5 {
		deltaLine := raw[i]
		deltaStart := raw[i+1]
		length := raw[i+2]
		tokenTypeIdx := raw[i+3]
		tokenModMask := raw[i+4]

		if deltaLine == 0 {
			char += deltaStart
		} else {
			line += deltaLine
			char = deltaStart
		}

		var modifiers []string
		for j, name := range lsp.SemanticTokenModifiers {
			if tokenModMask&(1<<j) != 0 {
				modifiers = append(modifiers, name)
			}
		}

		decoded = append(decoded, DecodedToken{
			Index:     i / 5,
			Line:      line + 1, // LSP uses 0-based indexing
			Char:      char + 1, // LSP uses 0-based indexing
			Length:    length,
			Type:      lsp.SemanticTokenTypes[tokenTypeIdx],
			Modifiers: modifiers,
		})
	}

	return decoded, nil
}

func assertToken(t *testing.T, token *DecodedToken, expectedLine, expectedChar, expectedLength uint32, expectedType string, expectedModifiers []string) {
	require.Equal(t, expectedLine, token.Line, "line mismatch (expected line %d)", expectedLine)
	require.Equal(t, expectedChar, token.Char, "char mismatch (expected char %d)", expectedChar)
	require.Equal(t, expectedLength, token.Length, "length mismatch")
	require.Equal(t, expectedType, token.Type, "type mismatch")
	require.ElementsMatch(t, expectedModifiers, token.Modifiers, "modifiers mismatch")
}
