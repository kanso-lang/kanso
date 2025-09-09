package parser

import (
	"testing"
)

func TestKeywordsAndIdentifiers(t *testing.T) {
	input := "fn let if else return contract require use struct writes reads ext mut customIdent"
	expected := []TokenType{
		FN, LET, IF, ELSE, RETURN, CONTRACT, REQUIRE,
		USE, STRUCT, WRITES, READS, EXT, MUT, IDENTIFIER,
	}

	scanner := NewScanner(input)
	tokens := scanner.ScanTokens()

	if len(tokens) < len(expected) {
		t.Fatalf("expected at least %d tokens, got %d", len(expected), len(tokens))
	}

	for i, exp := range expected {
		if tokens[i].Type != TokenType(exp) {
			t.Errorf("expected %s, got %s", exp, tokens[i].Type)
		}
	}
}

func TestNumbers(t *testing.T) {
	input := "42 0 12345 0x0 0x1F 0xABC"
	expected := []TokenType{NUMBER, NUMBER, NUMBER, HEX_NUMBER, HEX_NUMBER, HEX_NUMBER}

	scanner := NewScanner(input)
	tokens := scanner.ScanTokens()

	if len(tokens) < len(expected) {
		t.Fatalf("expected at least %d tokens, got %d", len(expected), len(tokens))
	}

	for i, exp := range expected {
		if tokens[i].Type != TokenType(exp) {
			t.Errorf("expected %s, got %s", exp, tokens[i].Type)
		}
	}
}

func TestStrings(t *testing.T) {
	input := `"hello" "world"`
	scanner := NewScanner(input)
	tokens := scanner.ScanTokens()

	if tokens[0].Type != STRING || tokens[0].Lexeme != "hello" {
		t.Errorf("expected STRING 'hello', got %s %s", tokens[0].Type, tokens[0].Lexeme)
	}
	if tokens[1].Type != STRING || tokens[1].Lexeme != "world" {
		t.Errorf("expected STRING 'world', got %s %s", tokens[1].Type, tokens[1].Lexeme)
	}
}

func TestOperatorsAndBrackets(t *testing.T) {
	input := `(){},.;+-*/! != == = < <= > >= # [ ] ::`
	expected := []TokenType{
		LEFT_PAREN, RIGHT_PAREN, LEFT_BRACE, RIGHT_BRACE, COMMA, DOT,
		SEMICOLON, PLUS, MINUS, STAR, SLASH, BANG, BANG_EQUAL,
		EQUAL_EQUAL, EQUAL, LESS, LESS_EQUAL, GREATER, GREATER_EQUAL,
		POUND, LEFT_BRACKET, RIGHT_BRACKET, DOUBLE_COLON,
	}
	expectedLexemes := []string{"(", ")", "{", "}", ",", ".", ";", "+", "-", "*", "/", "!", "!=", "==", "=", "<", "<=", ">", ">=", "#", "[", "]", "::"}

	scanner := NewScanner(input)
	tokens := scanner.ScanTokens()

	if len(tokens) < len(expected) {
		t.Fatalf("expected at least %d tokens, got %d", len(expected), len(tokens))
	}

	for i, exp := range expected {
		if tokens[i].Type != exp {
			t.Errorf("expected %s, got %s", exp, tokens[i].Type)
		}
		if tokens[i].Lexeme != expectedLexemes[i] {
			t.Errorf("expected lexeme '%s', got '%s'", expectedLexemes[i], tokens[i].Lexeme)
		}
	}
}

func TestSingleLineComments(t *testing.T) {
	input := `// comment line` + "\n" + `/// doc comment line`
	scanner := NewScanner(input)
	tokens := scanner.ScanTokens()

	if tokens[0].Type != COMMENT {
		t.Errorf("expected COMMENT, got %s", tokens[0].Type)
	}
	if tokens[1].Type != DOC_COMMENT {
		t.Errorf("expected DOC_COMMENT, got %s", tokens[1].Type)
	}
}

func TestBlockComments(t *testing.T) {
	input := `/* block comment */` + "\n" + `/** doc block comment */`
	scanner := NewScanner(input)
	tokens := scanner.ScanTokens()

	if tokens[0].Type != BLOCK_COMMENT {
		t.Errorf("expected BLOCK_COMMENT, got %s", tokens[0].Type)
	}
	if tokens[1].Type != DOC_COMMENT {
		t.Errorf("expected DOC_COMMENT, got %s", tokens[1].Type)
	}
}

func TestUnterminatedString(t *testing.T) {
	input := `"unterminated`
	scanner := NewScanner(input)
	_ = scanner.ScanTokens()

	if len(scanner.errors) == 0 {
		t.Fatal("expected an unterminated string error, got none")
	}

	asserError(t, scanner.errors[0], "Unterminated string.", 1, 1, 0)
}

func TestUnterminatedBlockComment(t *testing.T) {
	input := `/* unterminated comment`
	scanner := NewScanner(input)
	_ = scanner.ScanTokens()

	if len(scanner.errors) == 0 {
		t.Fatal("expected an unterminated block comment error, got none")
	}

	asserError(t, scanner.errors[0], "Unterminated block comment.", 1, 1, 0)
}

func TestInvalidHex(t *testing.T) {
	input := `0x`
	scanner := NewScanner(input)
	_ = scanner.ScanTokens()

	if len(scanner.errors) == 0 {
		t.Fatal("expected an invalid hex literal error, got none")
	}

	asserError(t, scanner.errors[0], "Invalid hex literal: expected hex digit after 0x", 1, 1, 0)
}

func asserError(t *testing.T, got ScanError, wantMessage string, wantLine, wantCol, wantOffset int) {
	if got.Message != wantMessage {
		t.Errorf("expected message '%s', got %q", wantMessage, got.Message)
	}
	if got.Position.Line != wantLine || got.Position.Column != wantCol || got.Position.Offset != wantOffset {
		t.Errorf("unexpected position: got line %d, column %d, offset %d",
			got.Position.Line, got.Position.Column, got.Position.Offset)
	}
}

func TestTokenPositions(t *testing.T) {
	input := "fn\nlet 123\n0x1F \"str\""
	scanner := NewScanner(input)
	tokens := scanner.ScanTokens()

	expected := []struct {
		typ    TokenType
		lexeme string
		line   int
		column int
	}{
		{FN, "fn", 1, 1},
		{LET, "let", 2, 1},
		{NUMBER, "123", 2, 5},
		{HEX_NUMBER, "0x1F", 3, 1},
		{STRING, "str", 3, 6},
	}

	for i, exp := range expected {
		if i >= len(tokens) {
			t.Fatalf("missing token at index %d", i)
		}
		tok := tokens[i]
		if tok.Type != exp.typ {
			t.Errorf("token %d: expected type %s, got %s", i, exp.typ, tok.Type)
		}
		if tok.Lexeme != exp.lexeme {
			t.Errorf("token %d: expected lexeme %q, got %q", i, exp.lexeme, tok.Lexeme)
		}
		if tok.Position.Line != exp.line {
			t.Errorf("token %d: expected line %d, got %d", i, exp.line, tok.Position.Line)
		}
		if tok.Position.Column != exp.column {
			t.Errorf("token %d: expected column %d, got %d", i, exp.column, tok.Position.Column)
		}
	}

	// Check that offsets strictly increase
	for i := 1; i < len(tokens); i++ {
		if tokens[i].Position.Offset <= tokens[i-1].Position.Offset {
			t.Errorf("token %d: expected offset to increase, got %d after %d",
				i, tokens[i].Position.Offset, tokens[i-1].Position.Offset)
		}
	}
}

func TestLogicalOperatorsAndCompound(t *testing.T) {
	input := "&& || += -= *= /= ** & |"
	expected := []TokenType{
		AND, OR, PLUS_EQUAL, MINUS_EQUAL, STAR_EQUAL, SLASH_EQUAL,
		STAR_STAR, AMPERSAND, PIPE,
	}
	expectedLexemes := []string{
		"&&", "||", "+=", "-=", "*=", "/=", "**", "&", "|",
	}

	scanner := NewScanner(input)
	tokens := scanner.ScanTokens()

	for i, exp := range expected {
		if i >= len(tokens) {
			t.Fatalf("missing token at index %d", i)
		}
		if tokens[i].Type != exp {
			t.Errorf("token %d: expected type %s, got %s", i, exp, tokens[i].Type)
		}
		if tokens[i].Lexeme != expectedLexemes[i] {
			t.Errorf("token %d: expected lexeme %q, got %q", i, expectedLexemes[i], tokens[i].Lexeme)
		}
	}
}

func TestMultilineUnterminatedBlockComment(t *testing.T) {
	input := `/* unterminated block
comment over multiple lines`
	scanner := NewScanner(input)
	_ = scanner.ScanTokens()

	if len(scanner.errors) == 0 {
		t.Fatal("expected unterminated block comment error, got none")
	}

	if scanner.errors[0].Message != "Unterminated block comment." {
		t.Errorf("expected unterminated block comment error, got %q", scanner.errors[0].Message)
	}
}

func TestMultilineUnterminatedString(t *testing.T) {
	input := `"unterminated string
that spans multiple lines`
	scanner := NewScanner(input)
	_ = scanner.ScanTokens()

	if len(scanner.errors) == 0 {
		t.Fatal("expected unterminated string error, got none")
	}

	if scanner.errors[0].Message != "Unterminated string." {
		t.Errorf("expected unterminated string error, got %q", scanner.errors[0].Message)
	}
}

func TestKeywordIdentifierBoundary(t *testing.T) {
	input := "publicToken public123 letValue functor"
	expected := []TokenType{
		IDENTIFIER, IDENTIFIER, IDENTIFIER, IDENTIFIER,
	}
	expectedLexemes := []string{
		"publicToken", "public123", "letValue", "functor",
	}

	scanner := NewScanner(input)
	tokens := scanner.ScanTokens()

	for i, exp := range expected {
		if i >= len(tokens) {
			t.Fatalf("missing token at index %d", i)
		}
		if tokens[i].Type != exp {
			t.Errorf("token %d: expected type %s, got %s", i, exp, tokens[i].Type)
		}
		if tokens[i].Lexeme != expectedLexemes[i] {
			t.Errorf("token %d: expected lexeme %q, got %q", i, expectedLexemes[i], tokens[i].Lexeme)
		}
	}
}

func TestMultilineBlockComment(t *testing.T) {
	input := `/* this is
a multiline
block comment */`
	scanner := NewScanner(input)
	tokens := scanner.ScanTokens()

	if len(tokens) == 0 || tokens[0].Type != BLOCK_COMMENT {
		t.Errorf("expected first token to be BLOCK_COMMENT, got %s", tokens[0].Type)
	}
}
