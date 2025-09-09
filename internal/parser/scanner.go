package parser

import (
	"fmt"
	"unicode"
)

type Token struct {
	Type     TokenType
	Lexeme   string
	Position Position
}

type Scanner struct {
	source      string
	tokens      []Token
	start       int
	current     int
	line        int
	startColumn int
	column      int
	offset      int
	errors      []ScanError
}

type ScanError struct {
	Message  string
	Position Position // line, column, offset
	Length   int      // optional: how many characters it covers
}

func NewScanner(source string) *Scanner {
	return &Scanner{
		source: source,
		line:   1,
		column: 1,
	}
}

func (s *Scanner) ScanTokens() []Token {
	for !s.isAtEnd() {
		s.start = s.current
		s.startColumn = s.column
		s.scanToken()
	}
	s.tokens = append(s.tokens, Token{Type: EOF, Position: Position{Line: s.line, Column: s.column, Offset: s.offset}})
	return s.tokens
}

func (s *Scanner) scanToken() {
	c := s.advance()
	switch c {
	// Simple single-character tokens
	case '(':
		s.addToken(LEFT_PAREN)
	case ')':
		s.addToken(RIGHT_PAREN)
	case '{':
		s.addToken(LEFT_BRACE)
	case '}':
		s.addToken(RIGHT_BRACE)
	case ',':
		s.addToken(COMMA)
	case '.':
		s.addToken(DOT)
	case ';':
		s.addToken(SEMICOLON)
	case '#':
		s.addToken(POUND)
	case '[':
		s.addToken(LEFT_BRACKET)
	case ']':
		s.addToken(RIGHT_BRACKET)

	// Operators with potential multi-character variants
	case '-':
		s.scanMinusOperator()
	case '+':
		s.scanPlusOperator()
	case ':':
		s.scanColonOperator()
	case '*':
		s.scanStarOperator()
	case '!':
		s.scanBangOperator()
	case '=':
		s.scanEqualOperator()
	case '&':
		s.scanAmpersandOperator()
	case '|':
		s.scanPipeOperator()
	case '<':
		s.scanLessOperator()
	case '>':
		s.scanGreaterOperator()
	case '/':
		s.scanSlashOperator()

	// Whitespace (ignored)
	case ' ', '\r', '\t':
		// Ignore whitespace
	case '\n':
		// Handled in advance()

	// String literals
	case '"':
		s.scanString()

	default:
		s.scanDefault(c)
	}
}

// Operator scanning methods for better organization

func (s *Scanner) scanMinusOperator() {
	if s.matchNext('-') {
		s.addToken(DECREMENT)
	} else if s.matchNext('=') {
		s.addToken(MINUS_EQUAL)
	} else if s.matchNext('>') {
		s.addToken(ARROW)
	} else {
		s.addToken(MINUS)
	}
}

func (s *Scanner) scanPlusOperator() {
	if s.matchNext('+') {
		s.addToken(INCREMENT)
	} else if s.matchNext('=') {
		s.addToken(PLUS_EQUAL)
	} else {
		s.addToken(PLUS)
	}
}

func (s *Scanner) scanColonOperator() {
	if s.matchNext(':') {
		s.addToken(DOUBLE_COLON)
	} else {
		s.addToken(COLON)
	}
}

func (s *Scanner) scanStarOperator() {
	if s.matchNext('*') {
		s.addToken(STAR_STAR)
	} else if s.matchNext('=') {
		s.addToken(STAR_EQUAL)
	} else {
		s.addToken(STAR)
	}
}

func (s *Scanner) scanBangOperator() {
	if s.matchNext('=') {
		s.addToken(BANG_EQUAL)
	} else {
		s.addToken(BANG)
	}
}

func (s *Scanner) scanEqualOperator() {
	if s.matchNext('=') {
		s.addToken(EQUAL_EQUAL)
	} else {
		s.addToken(EQUAL)
	}
}

func (s *Scanner) scanAmpersandOperator() {
	if s.matchNext('&') {
		s.addToken(AND)
	} else {
		s.addToken(AMPERSAND)
	}
}

func (s *Scanner) scanPipeOperator() {
	if s.matchNext('|') {
		s.addToken(OR)
	} else {
		s.addToken(PIPE)
	}
}

func (s *Scanner) scanLessOperator() {
	if s.matchNext('=') {
		s.addToken(LESS_EQUAL)
	} else {
		s.addToken(LESS)
	}
}

func (s *Scanner) scanGreaterOperator() {
	if s.matchNext('=') {
		s.addToken(GREATER_EQUAL)
	} else {
		s.addToken(GREATER)
	}
}

func (s *Scanner) scanSlashOperator() {
	if s.matchNext('=') {
		s.addToken(SLASH_EQUAL)
	} else if s.matchNext('/') {
		s.scanSingleLineComment()
	} else if s.matchNext('*') {
		s.scanBlockComment()
	} else {
		s.addToken(SLASH)
	}
}

func (s *Scanner) scanDefault(c byte) {
	if isDigit(c) {
		s.scanNumber()
	} else if isAlpha(c) {
		s.scanIdentifier()
	} else {
		s.reportError(fmt.Sprintf("Unexpected character: %q", c))
	}
}

func (s *Scanner) advance() byte {
	c := s.source[s.current]
	s.current++
	s.offset++
	if c == '\n' {
		s.line++
		s.column = 1
	} else {
		s.column++
	}
	return c
}

func (s *Scanner) matchNext(expected byte) bool {
	if s.isAtEnd() || s.source[s.current] != expected {
		return false
	}
	s.advance()
	return true
}

func (s *Scanner) peek() byte {
	if s.isAtEnd() {
		return 0
	}
	return s.source[s.current]
}

func (s *Scanner) addToken(tokenType TokenType) {
	text := s.source[s.start:s.current]
	s.tokens = append(s.tokens, Token{
		Type:   tokenType,
		Lexeme: text,
		Position: Position{
			Line:   s.line,
			Column: s.startColumn,
			Offset: s.start,
		},
	})
}

func (s *Scanner) reportError(message string) {
	s.errors = append(s.errors, ScanError{
		Message:  message,
		Position: Position{Line: s.line, Column: s.startColumn, Offset: s.start},
		Length:   s.current - s.start,
	})
}

func (s *Scanner) isAtEnd() bool {
	return s.current >= len(s.source)
}

// Helper functions.

func isDigit(c byte) bool {
	return '0' <= c && c <= '9'
}

func isAlpha(c byte) bool {
	return unicode.IsLetter(rune(c)) || c == '_'
}

func (s *Scanner) scanIdentifier() {
	for isAlpha(s.peek()) || isDigit(s.peek()) {
		s.advance()
	}
	text := s.source[s.start:s.current]

	// Don't consume the '!' as part of identifiers - let it be a separate token

	s.addToken(lookupIdentifier(text))
}

func (s *Scanner) scanNumber() {
	if s.peek() == 'x' || s.peek() == 'X' {
		s.advance()
		if !isHexDigit(s.peek()) {
			s.reportError("Invalid hex literal: expected hex digit after 0x")
			return
		}
		for isHexDigit(s.peek()) {
			s.advance()
		}
		s.addToken(HEX_NUMBER)
	} else {
		for isDigit(s.peek()) {
			s.advance()
		}
		s.addToken(NUMBER)
	}
}

func isHexDigit(c byte) bool {
	return ('0' <= c && c <= '9') ||
		('a' <= c && c <= 'f') ||
		('A' <= c && c <= 'F')
}

func (s *Scanner) scanString() {
	for s.peek() != '"' && !s.isAtEnd() {
		s.advance()
	}
	if s.isAtEnd() {
		s.reportError("Unterminated string.")
		return
	}
	s.advance()
	value := s.source[s.start+1 : s.current-1]
	s.tokens = append(s.tokens, Token{Type: STRING, Lexeme: value, Position: Position{
		Line: s.line, Column: s.startColumn, Offset: s.start},
	})
}

func lookupIdentifier(text string) TokenType {
	if t, ok := KEYWORDS[text]; ok {
		return t
	}
	return IDENTIFIER
}

func (s *Scanner) scanSingleLineComment() {
	for s.peek() != '\n' && !s.isAtEnd() {
		s.advance()
	}
	commentText := s.source[s.start:s.current]
	tokenType := COMMENT
	if len(commentText) >= 3 && commentText[:3] == "///" {
		tokenType = DOC_COMMENT
	}
	s.tokens = append(s.tokens, Token{Type: tokenType, Lexeme: commentText, Position: Position{
		Line: s.line, Column: s.startColumn, Offset: s.start}})
}

func (s *Scanner) scanBlockComment() {
	unterminated := true
	for !s.isAtEnd() {
		if s.peek() == '*' && s.peekNext() == '/' {
			s.advance() // *
			s.advance() // /
			unterminated = false
			break
		}
		if s.peek() == '\n' {
			s.line++
			s.column = 1
		} else {
			s.column++
		}
		s.advance()
	}

	commentText := s.source[s.start:s.current]
	if unterminated {
		s.reportError("Unterminated block comment.")
		return
	}

	tokenType := BLOCK_COMMENT
	if len(commentText) >= 3 && commentText[:3] == "/**" {
		tokenType = DOC_COMMENT
	}

	s.tokens = append(s.tokens, Token{
		Type:   tokenType,
		Lexeme: commentText,
		Position: Position{
			Line:   s.line,
			Column: s.startColumn,
			Offset: s.start,
		},
	})
}

func (s *Scanner) peekNext() byte {
	if s.current+1 >= len(s.source) {
		return 0
	}
	return s.source[s.current+1]
}
