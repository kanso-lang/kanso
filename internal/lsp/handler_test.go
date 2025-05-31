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

	assertToken(t, &decoded[0], 2, 1, 11, "modifier", nil)
	assertToken(t, &decoded[1], 3, 8, 5, "namespace", []string{"declaration"})
	assertToken(t, &decoded[2], 4, 9, 3, "namespace", nil)
	assertToken(t, &decoded[3], 4, 15, 6, "type", nil)
	assertToken(t, &decoded[4], 4, 23, 4, "type", nil)
	assertToken(t, &decoded[5], 5, 9, 5, "namespace", nil)
	assertToken(t, &decoded[6], 5, 17, 4, "type", nil)
	assertToken(t, &decoded[7], 5, 23, 5, "type", nil)
	assertToken(t, &decoded[8], 6, 9, 3, "namespace", nil)
	assertToken(t, &decoded[9], 6, 14, 5, "namespace", nil)
	assertToken(t, &decoded[10], 6, 22, 6, "type", nil)
	assertToken(t, &decoded[11], 7, 9, 3, "namespace", nil)
	assertToken(t, &decoded[12], 7, 14, 6, "namespace", nil)
	assertToken(t, &decoded[13], 9, 5, 8, "modifier", nil)
	assertToken(t, &decoded[14], 10, 12, 8, "type", []string{"declaration"})
	assertToken(t, &decoded[15], 11, 9, 4, "property", []string{"declaration"})
	assertToken(t, &decoded[16], 11, 15, 7, "type", nil)
	assertToken(t, &decoded[17], 12, 9, 2, "property", []string{"declaration"})
	assertToken(t, &decoded[18], 12, 13, 7, "type", nil)
	assertToken(t, &decoded[19], 13, 9, 5, "property", []string{"declaration"})
	assertToken(t, &decoded[20], 13, 16, 4, "type", nil)
	assertToken(t, &decoded[21], 16, 5, 8, "modifier", nil)
	assertToken(t, &decoded[22], 17, 12, 8, "type", []string{"declaration"})
	assertToken(t, &decoded[23], 18, 9, 5, "property", []string{"declaration"})
	assertToken(t, &decoded[24], 18, 16, 7, "type", nil)
	assertToken(t, &decoded[25], 19, 9, 7, "property", []string{"declaration"})
	assertToken(t, &decoded[26], 19, 18, 7, "type", nil)
	assertToken(t, &decoded[27], 20, 9, 5, "property", []string{"declaration"})
	assertToken(t, &decoded[28], 20, 16, 4, "type", nil)
	assertToken(t, &decoded[29], 23, 5, 10, "modifier", nil)
	assertToken(t, &decoded[30], 25, 12, 5, "type", []string{"declaration"})
	assertToken(t, &decoded[31], 26, 9, 8, "property", []string{"declaration"})
	assertToken(t, &decoded[32], 26, 19, 5, "type", nil)
	assertToken(t, &decoded[33], 27, 9, 10, "property", []string{"declaration"})
	assertToken(t, &decoded[34], 27, 21, 5, "type", nil)
	assertToken(t, &decoded[35], 28, 9, 12, "property", []string{"declaration"})
	assertToken(t, &decoded[36], 28, 23, 4, "type", nil)
	assertToken(t, &decoded[37], 29, 9, 4, "property", []string{"declaration"})
	assertToken(t, &decoded[38], 29, 15, 6, "type", nil)
	assertToken(t, &decoded[39], 30, 9, 6, "property", []string{"declaration"})
	assertToken(t, &decoded[40], 30, 17, 6, "type", nil)
	assertToken(t, &decoded[41], 31, 9, 8, "property", []string{"declaration"})
	assertToken(t, &decoded[42], 31, 19, 2, "type", nil)
	assertToken(t, &decoded[43], 34, 5, 9, "modifier", nil)
	assertToken(t, &decoded[44], 36, 9, 6, "function", []string{"declaration"})
	assertToken(t, &decoded[45], 36, 16, 4, "parameter", nil)
	assertToken(t, &decoded[46], 36, 22, 6, "type", nil)
	assertToken(t, &decoded[47], 36, 30, 6, "parameter", nil)
	assertToken(t, &decoded[48], 36, 38, 6, "type", nil)
	assertToken(t, &decoded[49], 36, 46, 14, "parameter", nil)
	assertToken(t, &decoded[50], 36, 62, 4, "type", nil)
	assertToken(t, &decoded[51], 36, 68, 8, "parameter", nil)
	assertToken(t, &decoded[52], 36, 78, 2, "type", nil)
	assertToken(t, &decoded[53], 36, 89, 5, "type", nil)
	assertToken(t, &decoded[54], 53, 16, 4, "function", []string{"declaration"})
	assertToken(t, &decoded[55], 53, 37, 5, "type", nil)
	assertToken(t, &decoded[56], 58, 16, 6, "function", []string{"declaration"})
	assertToken(t, &decoded[57], 58, 39, 5, "type", nil)
	assertToken(t, &decoded[58], 63, 16, 8, "function", []string{"declaration"})
	assertToken(t, &decoded[59], 68, 16, 11, "function", []string{"declaration"})
	assertToken(t, &decoded[60], 68, 42, 5, "type", nil)
	assertToken(t, &decoded[61], 73, 16, 9, "function", []string{"declaration"})
	assertToken(t, &decoded[62], 73, 26, 5, "parameter", nil)
	assertToken(t, &decoded[63], 73, 33, 7, "type", nil)
	assertToken(t, &decoded[64], 73, 54, 5, "type", nil)
	assertToken(t, &decoded[65], 74, 13, 1, "variable", []string{"declaration"})
	assertToken(t, &decoded[66], 79, 16, 8, "function", []string{"declaration"})
	assertToken(t, &decoded[67], 79, 25, 2, "parameter", nil)
	assertToken(t, &decoded[68], 79, 29, 7, "type", nil)
	assertToken(t, &decoded[69], 79, 38, 6, "parameter", nil)
	assertToken(t, &decoded[70], 79, 46, 4, "type", nil)
	assertToken(t, &decoded[71], 79, 65, 5, "type", nil)
	assertToken(t, &decoded[72], 87, 16, 12, "function", []string{"declaration"})
	assertToken(t, &decoded[73], 87, 29, 4, "parameter", nil)
	assertToken(t, &decoded[74], 87, 35, 7, "type", nil)
	assertToken(t, &decoded[75], 87, 44, 2, "parameter", nil)
	assertToken(t, &decoded[76], 87, 48, 7, "type", nil)
	assertToken(t, &decoded[77], 87, 57, 6, "parameter", nil)
	assertToken(t, &decoded[78], 87, 65, 4, "type", nil)
	assertToken(t, &decoded[79], 87, 84, 5, "type", nil)
	assertToken(t, &decoded[80], 90, 13, 1, "variable", []string{"declaration"})
	assertToken(t, &decoded[81], 92, 13, 15, "variable", []string{"declaration"})
	assertToken(t, &decoded[82], 93, 13, 20, "variable", []string{"declaration"})
	assertToken(t, &decoded[83], 104, 16, 7, "function", []string{"declaration"})
	assertToken(t, &decoded[84], 104, 24, 7, "parameter", nil)
	assertToken(t, &decoded[85], 104, 33, 7, "type", nil)
	assertToken(t, &decoded[86], 104, 42, 6, "parameter", nil)
	assertToken(t, &decoded[87], 104, 50, 4, "type", nil)
	assertToken(t, &decoded[88], 104, 69, 5, "type", nil)
	assertToken(t, &decoded[89], 105, 13, 1, "variable", []string{"declaration"})
	assertToken(t, &decoded[90], 106, 13, 1, "variable", []string{"declaration"})
	assertToken(t, &decoded[91], 115, 16, 9, "function", []string{"declaration"})
	assertToken(t, &decoded[92], 115, 26, 5, "parameter", nil)
	assertToken(t, &decoded[93], 115, 33, 7, "type", nil)
	assertToken(t, &decoded[94], 115, 42, 7, "parameter", nil)
	assertToken(t, &decoded[95], 115, 51, 7, "type", nil)
	assertToken(t, &decoded[96], 115, 72, 5, "type", nil)
	assertToken(t, &decoded[97], 116, 13, 1, "variable", []string{"declaration"})
	assertToken(t, &decoded[98], 121, 9, 11, "function", []string{"declaration"})
	assertToken(t, &decoded[99], 121, 21, 4, "parameter", nil)
	assertToken(t, &decoded[100], 121, 27, 7, "type", nil)
	assertToken(t, &decoded[101], 121, 36, 2, "parameter", nil)
	assertToken(t, &decoded[102], 121, 40, 7, "type", nil)
	assertToken(t, &decoded[103], 121, 49, 6, "parameter", nil)
	assertToken(t, &decoded[104], 121, 57, 4, "type", nil)
	assertToken(t, &decoded[105], 121, 70, 5, "type", nil)
	assertToken(t, &decoded[106], 122, 13, 1, "variable", []string{"declaration"})
	assertToken(t, &decoded[107], 123, 13, 8, "variable", []string{"declaration"})
	assertToken(t, &decoded[108], 127, 10, 8, "variable", nil)
	assertToken(t, &decoded[109], 128, 13, 6, "variable", []string{"declaration"})
	assertToken(t, &decoded[110], 135, 9, 13, "function", []string{"declaration"})
	assertToken(t, &decoded[111], 135, 23, 1, "parameter", nil)
	assertToken(t, &decoded[112], 135, 38, 5, "parameter", nil)
	assertToken(t, &decoded[113], 135, 45, 7, "type", nil)
	assertToken(t, &decoded[114], 141, 9, 4, "function", []string{"declaration"})
	assertToken(t, &decoded[115], 141, 14, 7, "parameter", nil)
	assertToken(t, &decoded[116], 141, 23, 7, "type", nil)
	assertToken(t, &decoded[117], 141, 32, 6, "parameter", nil)
	assertToken(t, &decoded[118], 141, 40, 4, "type", nil)
	assertToken(t, &decoded[119], 141, 53, 5, "type", nil)
	assertToken(t, &decoded[120], 142, 13, 1, "variable", []string{"declaration"})
	assertToken(t, &decoded[121], 145, 13, 15, "variable", []string{"declaration"})
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
