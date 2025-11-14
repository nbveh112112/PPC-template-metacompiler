use std::str::Chars;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum TokenizerError {
    #[error("Unterminated string literal at position {position}")]
    UnterminatedString { line: usize, column: usize, position: usize },
    #[error("Unterminated character literal at position {position}")]
    UnterminatedChar { line: usize, column: usize, position: usize},
    #[error("Unterminated comment at position {position}")]
    UnterminatedComment { line: usize, column: usize, position: usize },
    #[error("Invalid character '{character}' at position {position}")]
    InvalidCharacter { character: char, line: usize, column: usize, position: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Identifiers and literals
    Identifier(String),
    IntegerLiteral(String),
    FloatLiteral(String),
    CharLiteral(String),
    StringLiteral(String),

    // Keywords
    Auto, Break, Case, Char, Const, Continue, Default, Do, Double, Else, Enum, Extern,
    Float, For, Goto, If, Inline, Int, Long, Register, Restrict, Return, Short, Signed,
    Sizeof, Static, Struct, Switch, Typedef, Union, Unsigned, Void, Volatile, While,
    Alignas, Alignof, Atomic, Bool, Complex, Generic, Imaginary, Noreturn, StaticAssert,
    ThreadLocal, Template,

    // Operators
    Plus,          // +
    Minus,         // -
    Star,          // *
    Slash,         // /
    Percent,       // %
    Ampersand,     // &
    Pipe,          // |
    Caret,         // ^
    Tilde,         // ~
    Exclamation,   // !
    Question,      // ?
    Colon,         // :
    Semicolon,     // ;
    Comma,         // ,
    Dot,           // .
    Arrow,         // ->
    At,           // @

    // Assignment operators
    Assign,        // =
    PlusAssign,    // +=
    MinusAssign,   // -=
    StarAssign,    // *=
    SlashAssign,   // /=
    PercentAssign, // %=
    AndAssign,     // &=
    OrAssign,      // |=
    XorAssign,     // ^=
    ShlAssign,     // <<=
    ShrAssign,     // >>=

    // Comparison operators
    Equal,         // ==
    NotEqual,      // !=
    Less,          // <
    LessEqual,     // <=
    Greater,       // >
    GreaterEqual,  // >=

    // Increment/Decrement
    Increment,     // ++
    Decrement,     // --

    // Bitwise shifts
    Shl,           // <<
    Shr,           // >>

    // Logical operators
    And,           // &&
    Or,            // ||

    // Brackets
    LeftParen,     // (
    RightParen,    // )
    LeftBracket,   // [
    RightBracket,  // ]
    LeftBrace,     // {
    RightBrace,    // }

    // Whitespace (preserved for formatting)
    Whitespace(String),

    // Comments
    Comment(String),


    // End of file
    Eof,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TokenInfo {
    pub token: Token,
    pub line: usize,
    pub column: usize,
    pub position: usize,
}

#[allow(dead_code)]
pub struct Tokenizer<'a> {
    chars: Chars<'a>,
    current: Option<char>,
    position: usize,
    line: usize,
    column: usize,
    source: &'a str,
}

impl<'a> Tokenizer<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut tokenizer = Tokenizer {
            chars: source.chars(),
            current: None,
            position: 0,
            line: 1,
            column: 1,
            source,
        };
        tokenizer.advance();
        tokenizer
    }

    fn advance(&mut self) {
        self.current = self.chars.next();
        if let Some(c) = self.current {
            self.position += c.len_utf8();
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }

    pub fn tokenize(&mut self) -> Result<Vec<TokenInfo>, TokenizerError> {
        let mut tokens = Vec::new();

        while self.current.is_some() {
            let start_pos = self.position;
            let start_line = self.line;
            let start_col = self.column;

            let token = if let Some(c) = self.current {
                match c {
                    // Whitespace
                    c if c.is_whitespace() => {
                        let whitespace = self.read_while(|ch| ch.is_whitespace());
                        Token::Whitespace(whitespace)
                    }

                    // Comments
                    '/' => {
                        if let Some('/') = self.peek() {
                            self.advance(); // consume second '/'
                            let comment = self.read_while(|ch| ch != '\n');
                            Token::Comment(format!("//{}", comment))
                        } else if let Some('*') = self.peek() {
                            self.advance(); // consume '*'
                            self.read_multiline_comment(start_pos)?
                        } else {
                            self.read_operator()
                        }
                    }

                    // Preprocessor directives
                    '#' => {
                        return Err(TokenizerError::InvalidCharacter {character: '#', line: self.line, column: self.column, position: self.column});
                    }

                    // String literals
                    '"' => self.read_string_literal()?,

                    // Character literals
                    '\'' => self.read_char_literal()?,

                    // Identifiers and keywords
                    c if c.is_alphabetic() || c == '_' => self.read_identifier(),

                    // Numbers
                    c if c.is_ascii_digit() => self.read_number(),

                    // Operators and punctuation
                    _ => self.read_operator(),
                }
            } else {
                break;
            };

            tokens.push(TokenInfo {
                token,
                line: start_line,
                column: start_col,
                position: start_pos,
            });
        }

        tokens.push(TokenInfo {
            token: Token::Eof,
            line: self.line,
            column: self.column,
            position: self.position,
        });

        Ok(tokens)
    }

    fn read_while<F>(&mut self, mut predicate: F) -> String
    where
        F: FnMut(char) -> bool,
    {
        let mut result = String::new();
        while let Some(c) = self.current {
            if predicate(c) {
                result.push(c);
                self.advance();
            } else {
                break;
            }
        }
        result
    }

    fn read_identifier(&mut self) -> Token {
        let ident = self.read_while(|c| c.is_alphanumeric() || c == '_');

        // Check for keywords
        match ident.as_str() {
            "auto" => Token::Auto,
            "break" => Token::Break,
            "case" => Token::Case,
            "char" => Token::Char,
            "const" => Token::Const,
            "continue" => Token::Continue,
            "default" => Token::Default,
            "do" => Token::Do,
            "double" => Token::Double,
            "else" => Token::Else,
            "enum" => Token::Enum,
            "extern" => Token::Extern,
            "float" => Token::Float,
            "for" => Token::For,
            "goto" => Token::Goto,
            "if" => Token::If,
            "inline" => Token::Inline,
            "int" => Token::Int,
            "long" => Token::Long,
            "register" => Token::Register,
            "restrict" => Token::Restrict,
            "return" => Token::Return,
            "short" => Token::Short,
            "signed" => Token::Signed,
            "sizeof" => Token::Sizeof,
            "static" => Token::Static,
            "struct" => Token::Struct,
            "switch" => Token::Switch,
            "typedef" => Token::Typedef,
            "union" => Token::Union,
            "unsigned" => Token::Unsigned,
            "void" => Token::Void,
            "volatile" => Token::Volatile,
            "while" => Token::While,
            // "_Alignas" => Token::Alignas,
            // "_Alignof" => Token::Alignof,
            // "_Atomic" => Token::Atomic,
            // "_Bool" => Token::Bool,
            // "_Complex" => Token::Complex,
            // "_Generic" => Token::Generic,
            // "_Imaginary" => Token::Imaginary,
            // "_Noreturn" => Token::Noreturn,
            // "_Static_assert" => Token::StaticAssert,
            // "_Thread_local" => Token::ThreadLocal,
            "template" => Token::Template,
            _ => Token::Identifier(ident),
        }
    }

    fn read_number(&mut self) -> Token {
        let mut number = String::new();

        // Read integer part
        number.push_str(&self.read_while(|c| c.is_ascii_digit()));

        // Read fractional part
        // Check for floating point
        if self.current == Some('.') {
            number.push('.');
            self.advance();
            number.push_str(&self.read_while(|c| c.is_ascii_digit()));
            Token::FloatLiteral(number)
        } else {
            Token::IntegerLiteral(number)
        }
    }

    fn read_string_literal(&mut self) -> Result<Token, TokenizerError> {
        self.advance(); // consume opening quote
        let mut string = String::new();
        let start_pos = self.position;
        let start_line = self.line;
        let start_col = self.column;

        while let Some(c) = self.current {
            match c {
                '"' => {
                    self.advance();
                    return Ok(Token::StringLiteral(string));
                }
                '\\' => {
                    self.advance(); // consume backslash
                    if let Some(escaped) = self.current {
                        string.push('\\');
                        string.push(escaped);
                        self.advance();
                    }
                }
                _ => {
                    string.push(c);
                    self.advance();
                }
            }
        }

        Err(TokenizerError::UnterminatedString { line: start_line, column: start_col, position: start_pos })
    }

    fn read_char_literal(&mut self) -> Result<Token, TokenizerError> {
        self.advance(); // consume opening quote
        let mut char_lit = String::new();
        let start_pos = self.position;
        let start_line = self.line;
        let start_col = self.column;

        while let Some(c) = self.current {
            match c {
                '\'' => {
                    self.advance();
                    return Ok(Token::CharLiteral(char_lit));
                }
                '\\' => {
                    self.advance(); // consume backslash
                    if let Some(escaped) = self.current {
                        char_lit.push('\\');
                        char_lit.push(escaped);
                        self.advance();
                    }
                }
                _ => {
                    char_lit.push(c);
                    self.advance();
                }
            }
        }

        Err(TokenizerError::UnterminatedChar { line: start_line, column: start_col, position: start_pos })
    }

    fn read_multiline_comment(&mut self, start_pos: usize) -> Result<Token, TokenizerError> {
        let mut comment = String::from("/*");
        self.advance(); // consume '*' (we already consumed the first '/' and '*')

        let start_line = self.line;
        let start_col = self.column;

        while let Some(c) = self.current {
            if c == '*' {
                self.advance();
                if let Some('/') = self.current {
                    self.advance();
                    comment.push('*');
                    comment.push('/');
                    return Ok(Token::Comment(comment));
                } else {
                    comment.push('*');
                }
            } else {
                comment.push(c);
                self.advance();
            }
        }

        Err(TokenizerError::UnterminatedComment { line: start_line, column: start_col, position: start_pos })
    }

    fn read_operator(&mut self) -> Token {
        match self.current {
            Some('+') => {
                self.advance();
                match self.current {
                    Some('+') => {
                        self.advance();
                        Token::Increment
                    }
                    Some('=') => {
                        self.advance();
                        Token::PlusAssign
                    }
                    _ => Token::Plus,
                }
            }
            Some('-') => {
                self.advance();
                match self.current {
                    Some('-') => {
                        self.advance();
                        Token::Decrement
                    }
                    Some('=') => {
                        self.advance();
                        Token::MinusAssign
                    }
                    Some('>') => {
                        self.advance();
                        Token::Arrow
                    }
                    _ => Token::Minus,
                }
            }
            Some('*') => {
                self.advance();
                if self.current == Some('=') {
                    self.advance();
                    Token::StarAssign
                } else {
                    Token::Star
                }
            }
            Some('/') => {
                self.advance();
                if self.current == Some('=') {
                    self.advance();
                    Token::SlashAssign
                } else {
                    Token::Slash
                }
            }
            Some('%') => {
                self.advance();
                if self.current == Some('=') {
                    self.advance();
                    Token::PercentAssign
                } else {
                    Token::Percent
                }
            }
            Some('&') => {
                self.advance();
                if self.current == Some('&') {
                    self.advance();
                    Token::And
                } else if self.current == Some('=') {
                    self.advance();
                    Token::AndAssign
                } else {
                    Token::Ampersand
                }
            }
            Some('|') => {
                self.advance();
                if self.current == Some('|') {
                    self.advance();
                    Token::Or
                } else if self.current == Some('=') {
                    self.advance();
                    Token::OrAssign
                } else {
                    Token::Pipe
                }
            }
            Some('^') => {
                self.advance();
                if self.current == Some('=') {
                    self.advance();
                    Token::XorAssign
                } else {
                    Token::Caret
                }
            }
            Some('~') => {
                self.advance();
                Token::Tilde
            }
            Some('!') => {
                self.advance();
                if self.current == Some('=') {
                    self.advance();
                    Token::NotEqual
                } else {
                    Token::Exclamation
                }
            }
            Some('=') => {
                self.advance();
                if self.current == Some('=') {
                    self.advance();
                    Token::Equal
                } else {
                    Token::Assign
                }
            }
            Some('<') => {
                self.advance();
                match self.current {
                    Some('<') => {
                        self.advance();
                        if self.current == Some('=') {
                            self.advance();
                            Token::ShlAssign
                        } else {
                            Token::Shl
                        }
                    }
                    Some('=') => {
                        self.advance();
                        Token::LessEqual
                    }
                    _ => Token::Less,
                }
            }
            Some('>') => {
                self.advance();
                match self.current {
                    Some('>') => {
                        self.advance();
                        if self.current == Some('=') {
                            self.advance();
                            Token::ShrAssign
                        } else {
                            Token::Shr
                        }
                    }
                    Some('=') => {
                        self.advance();
                        Token::GreaterEqual
                    }
                    _ => Token::Greater,
                }
            }
            Some('?') => {
                self.advance();
                Token::Question
            }
            Some(':') => {
                self.advance();
                Token::Colon
            }
            Some(';') => {
                self.advance();
                Token::Semicolon
            }
            Some(',') => {
                self.advance();
                Token::Comma
            }
            Some('.') => {
                self.advance();
                Token::Dot
            }
            Some('(') => {
                self.advance();
                Token::LeftParen
            }
            Some(')') => {
                self.advance();
                Token::RightParen
            }
            Some('[') => {
                self.advance();
                Token::LeftBracket
            }
            Some(']') => {
                self.advance();
                Token::RightBracket
            }
            Some('{') => {
                self.advance();
                Token::LeftBrace
            }
            Some('}') => {
                self.advance();
                Token::RightBrace
            }
            Some('@') => {
                self.advance();
                Token::At
            }
            Some(c) => {
                self.advance();
                Token::Identifier(c.to_string()) // Fallback for unknown characters
            }
            None => Token::Eof,
        }
    }
}

// Helper function to tokenize a string
pub fn tokenize(source: &str) -> Result<Vec<TokenInfo>, TokenizerError> {
    let mut tokenizer = Tokenizer::new(source);
    tokenizer.tokenize()
}