use memchr::memrchr3_iter;
use ruff_text_size::{TextLen, TextRange, TextSize};
use unic_ucd_ident::{is_xid_continue, is_xid_start};

use crate::{is_python_whitespace, Cursor};

/// Searches for the first non-trivia character in `range`.
///
/// The search skips over any whitespace and comments.
///
/// Returns `Some` if the range contains any non-trivia character. The first item is the absolute offset
/// of the character, the second item the non-trivia character.
///
/// Returns `None` if the range is empty or only contains trivia (whitespace or comments).
pub fn first_non_trivia_token(offset: TextSize, code: &str) -> Option<Token> {
    SimpleTokenizer::starts_at(offset, code)
        .skip_trivia()
        .next()
}

/// Returns the first non-trivia token right before `offset` or `None` if at the start of the file
/// or all preceding tokens are trivia tokens.
///
/// ## Notes
///
/// Prefer [`first_non_trivia_token`] whenever possible because reverse lookup is expensive because of comments.
pub fn first_non_trivia_token_rev(offset: TextSize, code: &str) -> Option<Token> {
    SimpleTokenizer::up_to(offset, code)
        .skip_trivia()
        .next_back()
}

/// Returns the number of newlines between `offset` and the first non whitespace character in the source code.
pub fn lines_before(offset: TextSize, code: &str) -> u32 {
    let tokens = SimpleTokenizer::up_to(offset, code);
    let mut newlines = 0u32;

    for token in tokens.rev() {
        match token.kind() {
            TokenKind::Newline => {
                newlines += 1;
            }
            TokenKind::Whitespace => {
                // ignore
            }
            _ => {
                break;
            }
        }
    }

    newlines
}

/// Counts the empty lines between `offset` and the first non-whitespace character.
pub fn lines_after(offset: TextSize, code: &str) -> u32 {
    let tokens = SimpleTokenizer::starts_at(offset, code);
    let mut newlines = 0u32;

    for token in tokens {
        match token.kind() {
            TokenKind::Newline => {
                newlines += 1;
            }
            TokenKind::Whitespace => {
                // ignore
            }
            _ => {
                break;
            }
        }
    }

    newlines
}

/// Returns the position after skipping any trailing trivia up to, but not including the newline character.
pub fn skip_trailing_trivia(offset: TextSize, code: &str) -> TextSize {
    let tokenizer = SimpleTokenizer::starts_at(offset, code);

    for token in tokenizer {
        match token.kind() {
            TokenKind::Whitespace | TokenKind::Comment | TokenKind::Continuation => {
                // No op
            }
            _ => {
                return token.start();
            }
        }
    }

    offset
}

fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || is_non_ascii_identifier_start(c)
}

// Checks if the character c is a valid continuation character as described
// in https://docs.python.org/3/reference/lexical_analysis.html#identifiers
fn is_identifier_continuation(c: char) -> bool {
    if c.is_ascii() {
        matches!(c, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9')
    } else {
        is_xid_continue(c)
    }
}

fn is_non_ascii_identifier_start(c: char) -> bool {
    is_xid_start(c)
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Token {
    pub kind: TokenKind,
    pub range: TextRange,
}

impl Token {
    pub const fn kind(&self) -> TokenKind {
        self.kind
    }

    #[allow(unused)]
    pub const fn range(&self) -> TextRange {
        self.range
    }

    pub const fn start(&self) -> TextSize {
        self.range.start()
    }

    pub const fn end(&self) -> TextSize {
        self.range.end()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TokenKind {
    /// A comment, not including the trailing new line.
    Comment,

    /// Sequence of ' ' or '\t'
    Whitespace,

    /// Start or end of the file
    EndOfFile,

    /// `\\`
    Continuation,

    /// `\n` or `\r` or `\r\n`
    Newline,

    /// `(`
    LParen,

    /// `)`
    RParen,

    /// `{`
    LBrace,

    /// `}`
    RBrace,

    /// `[`
    LBracket,

    /// `]`
    RBracket,

    /// `,`
    Comma,

    /// `:`
    Colon,

    /// '/'
    Slash,

    /// '*'
    Star,

    /// `.`.
    Dot,

    /// `else`
    Else,

    /// `if`
    If,

    /// `in`
    In,

    /// `as`
    As,

    /// `match`
    Match,

    /// `with`
    With,

    /// `async`
    Async,

    /// Any other non trivia token.
    Other,

    /// Returned for each character after [`TokenKind::Other`] has been returned once.
    Bogus,
}

impl TokenKind {
    const fn from_non_trivia_char(c: char) -> TokenKind {
        match c {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            '/' => TokenKind::Slash,
            '*' => TokenKind::Star,
            '.' => TokenKind::Dot,
            _ => TokenKind::Other,
        }
    }

    const fn is_trivia(self) -> bool {
        matches!(
            self,
            TokenKind::Whitespace
                | TokenKind::Newline
                | TokenKind::Comment
                | TokenKind::Continuation
        )
    }
}

/// Simple zero allocation tokenizer for tokenizing trivia (and some tokens).
///
/// The tokenizer must start at an offset that is trivia (e.g. not inside of a multiline string).
///
/// The tokenizer doesn't guarantee any correctness after it returned a [`TokenKind::Other`]. That's why it
/// will return [`TokenKind::Bogus`] for every character after until it reaches the end of the file.
pub struct SimpleTokenizer<'a> {
    offset: TextSize,
    back_offset: TextSize,
    /// `true` when it is known that the current `back` line has no comment for sure.
    back_line_has_no_comment: bool,
    bogus: bool,
    source: &'a str,
    cursor: Cursor<'a>,
}

impl<'a> SimpleTokenizer<'a> {
    pub fn new(source: &'a str, range: TextRange) -> Self {
        Self {
            offset: range.start(),
            back_offset: range.end(),
            back_line_has_no_comment: false,
            bogus: false,
            source,
            cursor: Cursor::new(&source[range]),
        }
    }

    pub fn starts_at(offset: TextSize, source: &'a str) -> Self {
        let range = TextRange::new(offset, source.text_len());
        Self::new(source, range)
    }

    /// Creates a tokenizer that lexes tokens from the start of `source` up to `offset`.
    pub fn up_to(offset: TextSize, source: &'a str) -> Self {
        Self::new(source, TextRange::up_to(offset))
    }

    /// Creates a tokenizer that lexes tokens from the start of `source` up to `offset`, and informs
    /// the lexer that the line at `offset` contains no comments. This can significantly speed up backwards lexing
    /// because the lexer doesn't need to scan for comments.
    pub fn up_to_without_back_comment(offset: TextSize, source: &'a str) -> Self {
        let mut tokenizer = Self::up_to(offset, source);
        tokenizer.back_line_has_no_comment = true;
        tokenizer
    }

    fn to_keyword_or_other(&self, range: TextRange) -> TokenKind {
        let source = &self.source[range];
        match source {
            "as" => TokenKind::As,
            "async" => TokenKind::Async,
            "else" => TokenKind::Else,
            "if" => TokenKind::If,
            "in" => TokenKind::In,
            "match" => TokenKind::Match, // Match is a soft keyword that depends on the context but we can always lex it as a keyword and leave it to the caller (parser) to decide if it should be handled as an identifier or keyword.
            "with" => TokenKind::With,
            // ...,
            _ => TokenKind::Other, // Potentially an identifier, but only if it isn't a string prefix. We can ignore this for now https://docs.python.org/3/reference/lexical_analysis.html#string-and-bytes-literals
        }
    }

    fn next_token(&mut self) -> Token {
        self.cursor.start_token();

        let Some(first) = self.cursor.bump() else {
            return Token {
                kind: TokenKind::EndOfFile,
                range: TextRange::empty(self.offset),
            };
        };

        if self.bogus {
            let token = Token {
                kind: TokenKind::Bogus,
                range: TextRange::at(self.offset, first.text_len()),
            };

            self.offset += first.text_len();
            return token;
        }

        let kind = match first {
            ' ' | '\t' => {
                self.cursor.eat_while(|c| matches!(c, ' ' | '\t'));
                TokenKind::Whitespace
            }

            '\n' => TokenKind::Newline,

            '\r' => {
                self.cursor.eat_char('\n');
                TokenKind::Newline
            }

            '#' => {
                self.cursor.eat_while(|c| !matches!(c, '\n' | '\r'));
                TokenKind::Comment
            }

            '\\' => TokenKind::Continuation,

            c => {
                let kind = if is_identifier_start(c) {
                    self.cursor.eat_while(is_identifier_continuation);
                    let token_len = self.cursor.token_len();

                    let range = TextRange::at(self.offset, token_len);
                    self.to_keyword_or_other(range)
                } else {
                    TokenKind::from_non_trivia_char(c)
                };

                if kind == TokenKind::Other {
                    self.bogus = true;
                }
                kind
            }
        };

        let token_len = self.cursor.token_len();

        let token = Token {
            kind,
            range: TextRange::at(self.offset, token_len),
        };

        self.offset += token_len;

        token
    }

    /// Returns the next token from the back. Prefer iterating forwards. Iterating backwards is significantly more expensive
    /// because it needs to check if the line has any comments when encountering any non-trivia token.
    pub fn next_token_back(&mut self) -> Token {
        self.cursor.start_token();

        let Some(last) = self.cursor.bump_back() else {
            return Token {
                kind: TokenKind::EndOfFile,
                range: TextRange::empty(self.back_offset),
            };
        };

        if self.bogus {
            let token = Token {
                kind: TokenKind::Bogus,
                range: TextRange::at(self.back_offset - last.text_len(), last.text_len()),
            };

            self.back_offset -= last.text_len();
            return token;
        }

        let kind = match last {
            // This may not be 100% correct because it will lex-out trailing whitespace from a comment
            // as whitespace rather than being part of the token. This shouldn't matter for what we use the lexer for.
            ' ' | '\t' => {
                self.cursor.eat_back_while(|c| matches!(c, ' ' | '\t'));
                TokenKind::Whitespace
            }

            '\r' => {
                self.back_line_has_no_comment = false;
                TokenKind::Newline
            }

            '\n' => {
                self.back_line_has_no_comment = false;
                self.cursor.eat_char_back('\r');
                TokenKind::Newline
            }

            // Empty comment (could also be a comment nested in another comment, but this shouldn't matter for what we use the lexer for)
            '#' => TokenKind::Comment,

            // For all other tokens, test if the character isn't part of a comment.
            c => {
                // Skip the test whether there's a preceding comment if it has been performed before.
                let comment_offset = if self.back_line_has_no_comment {
                    None
                } else {
                    let bytes = self.cursor.chars().as_str().as_bytes();
                    let mut line_start = 0;
                    let mut last_comment_offset = None;

                    // Find the start of the line, or any potential comments.
                    for index in memrchr3_iter(b'\n', b'\r', b'#', bytes) {
                        if bytes[index] == b'#' {
                            // Potentially a comment, but not guaranteed
                            last_comment_offset = Some(index);
                        } else {
                            line_start = index + 1;
                            break;
                        }
                    }

                    // Verify if this is indeed a comment. Doing this only when we've found a comment is significantly
                    // faster because comments are rare.
                    last_comment_offset.filter(|last_comment_offset| {
                        let before_comment =
                            &self.cursor.chars().as_str()[line_start..*last_comment_offset];

                        before_comment.chars().all(|c| {
                            is_python_whitespace(c)
                                || TokenKind::from_non_trivia_char(c) != TokenKind::Other
                        })
                    })
                };

                // From here on it is guaranteed that this line has no other comment.
                self.back_line_has_no_comment = true;

                if let Some(comment_offset) = comment_offset {
                    let comment_length = self.cursor.chars().as_str().len() - comment_offset;
                    // It is a comment, bump all tokens
                    for _ in 0..comment_length {
                        self.cursor.bump_back().unwrap();
                    }

                    TokenKind::Comment
                } else if c == '\\' {
                    TokenKind::Continuation
                } else {
                    let kind = if is_identifier_continuation(c) {
                        // if we only have identifier continuations but no start (e.g. 555) we
                        // don't want to consume the chars, so in that case, we want to rewind the
                        // cursor to here
                        let savepoint = self.cursor.clone();
                        self.cursor.eat_back_while(is_identifier_continuation);

                        let token_len = self.cursor.token_len();
                        let range = TextRange::at(self.back_offset - token_len, token_len);

                        if self.source[range]
                            .chars()
                            .next()
                            .is_some_and(is_identifier_start)
                        {
                            self.to_keyword_or_other(range)
                        } else {
                            self.cursor = savepoint;
                            TokenKind::Other
                        }
                    } else {
                        TokenKind::from_non_trivia_char(c)
                    };

                    if kind == TokenKind::Other {
                        self.bogus = true;
                    }

                    kind
                }
            }
        };

        let token_len = self.cursor.token_len();

        let start = self.back_offset - token_len;

        let token = Token {
            kind,
            range: TextRange::at(start, token_len),
        };

        self.back_offset = start;

        token
    }

    pub fn skip_trivia(self) -> impl Iterator<Item = Token> + DoubleEndedIterator + 'a {
        self.filter(|t| !t.kind().is_trivia())
    }
}

impl Iterator for SimpleTokenizer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token();

        if token.kind == TokenKind::EndOfFile {
            None
        } else {
            Some(token)
        }
    }
}

impl DoubleEndedIterator for SimpleTokenizer<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let token = self.next_token_back();

        if token.kind == TokenKind::EndOfFile {
            None
        } else {
            Some(token)
        }
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use ruff_text_size::{TextLen, TextRange, TextSize};

    use crate::tokenizer::{lines_after, lines_before, SimpleTokenizer, Token};

    struct TokenizationTestCase {
        source: &'static str,
        range: TextRange,
        tokens: Vec<Token>,
    }

    impl TokenizationTestCase {
        fn assert_reverse_tokenization(&self) {
            let mut backwards = self.tokenize_reverse();

            // Re-reverse to get the tokens in forward order.
            backwards.reverse();

            assert_eq!(&backwards, &self.tokens);
        }

        fn tokenize_reverse(&self) -> Vec<Token> {
            SimpleTokenizer::new(self.source, self.range)
                .rev()
                .collect()
        }

        fn tokens(&self) -> &[Token] {
            &self.tokens
        }
    }

    fn tokenize_range(source: &'static str, range: TextRange) -> TokenizationTestCase {
        let tokens: Vec<_> = SimpleTokenizer::new(source, range).collect();

        TokenizationTestCase {
            source,
            range,
            tokens,
        }
    }

    fn tokenize(source: &'static str) -> TokenizationTestCase {
        tokenize_range(source, TextRange::new(TextSize::new(0), source.text_len()))
    }

    #[test]
    fn tokenize_trivia() {
        let source = "# comment\n    # comment";

        let test_case = tokenize(source);

        assert_debug_snapshot!(test_case.tokens());
        test_case.assert_reverse_tokenization();
    }

    #[test]
    fn tokenize_parentheses() {
        let source = "([{}])";

        let test_case = tokenize(source);

        assert_debug_snapshot!(test_case.tokens());
        test_case.assert_reverse_tokenization();
    }

    #[test]
    fn tokenize_comma() {
        let source = ",,,,";

        let test_case = tokenize(source);

        assert_debug_snapshot!(test_case.tokens());
        test_case.assert_reverse_tokenization();
    }

    #[test]
    fn tokenize_continuation() {
        let source = "( \\\n )";

        let test_case = tokenize(source);

        assert_debug_snapshot!(test_case.tokens());
        test_case.assert_reverse_tokenization();
    }

    #[test]
    fn tricky_unicode() {
        let source = "មុ";

        let test_case = tokenize(source);
        assert_debug_snapshot!(test_case.tokens());
        test_case.assert_reverse_tokenization();
    }

    #[test]
    fn identifier_ending_in_non_start_char() {
        let source = "i5";

        let test_case = tokenize(source);
        assert_debug_snapshot!(test_case.tokens());
        test_case.assert_reverse_tokenization();
    }

    #[test]
    fn ignore_word_with_only_id_continuing_chars() {
        let source = "555";

        let test_case = tokenize(source);
        assert_debug_snapshot!(test_case.tokens());

        // note: not reversible: [other, bogus, bogus] vs [bogus, bogus, other]
    }

    #[test]
    fn tokenize_multichar() {
        let source = "if in else match";

        let test_case = tokenize(source);

        assert_debug_snapshot!(test_case.tokens());
        test_case.assert_reverse_tokenization();
    }

    #[test]
    fn tokenize_substring() {
        let source = "('some string') # comment";

        let test_case =
            tokenize_range(source, TextRange::new(TextSize::new(14), source.text_len()));

        assert_debug_snapshot!(test_case.tokens());
        test_case.assert_reverse_tokenization();
    }

    #[test]
    fn tokenize_slash() {
        let source = r#" # trailing positional comment
        # Positional arguments only after here
        ,/"#;

        let test_case = tokenize(source);

        assert_debug_snapshot!(test_case.tokens());
        test_case.assert_reverse_tokenization();
    }

    #[test]
    fn tokenize_bogus() {
        let source = r#"# leading comment
        "a string"
        a = (10)"#;

        let test_case = tokenize(source);

        assert_debug_snapshot!(test_case.tokens());
        assert_debug_snapshot!("Reverse", test_case.tokenize_reverse());
    }

    #[test]
    fn lines_before_empty_string() {
        assert_eq!(lines_before(TextSize::new(0), ""), 0);
    }

    #[test]
    fn lines_before_in_the_middle_of_a_line() {
        assert_eq!(lines_before(TextSize::new(4), "a = 20"), 0);
    }

    #[test]
    fn lines_before_on_a_new_line() {
        assert_eq!(lines_before(TextSize::new(7), "a = 20\nb = 10"), 1);
    }

    #[test]
    fn lines_before_multiple_leading_newlines() {
        assert_eq!(lines_before(TextSize::new(9), "a = 20\n\r\nb = 10"), 2);
    }

    #[test]
    fn lines_before_with_comment_offset() {
        assert_eq!(lines_before(TextSize::new(8), "a = 20\n# a comment"), 0);
    }

    #[test]
    fn lines_before_with_trailing_comment() {
        assert_eq!(
            lines_before(TextSize::new(22), "a = 20 # some comment\nb = 10"),
            1
        );
    }

    #[test]
    fn lines_before_with_comment_only_line() {
        assert_eq!(
            lines_before(TextSize::new(22), "a = 20\n# some comment\nb = 10"),
            1
        );
    }

    #[test]
    fn lines_after_empty_string() {
        assert_eq!(lines_after(TextSize::new(0), ""), 0);
    }

    #[test]
    fn lines_after_in_the_middle_of_a_line() {
        assert_eq!(lines_after(TextSize::new(4), "a = 20"), 0);
    }

    #[test]
    fn lines_after_before_a_new_line() {
        assert_eq!(lines_after(TextSize::new(6), "a = 20\nb = 10"), 1);
    }

    #[test]
    fn lines_after_multiple_newlines() {
        assert_eq!(lines_after(TextSize::new(6), "a = 20\n\r\nb = 10"), 2);
    }

    #[test]
    fn lines_after_before_comment_offset() {
        assert_eq!(lines_after(TextSize::new(7), "a = 20 # a comment\n"), 0);
    }

    #[test]
    fn lines_after_with_comment_only_line() {
        assert_eq!(
            lines_after(TextSize::new(6), "a = 20\n# some comment\nb = 10"),
            1
        );
    }
}
