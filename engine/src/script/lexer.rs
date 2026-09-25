use super::ScriptError;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Number(f64),
    Str(String),
    Ident(String),

    Let,
    Fn,
    If,
    Else,
    While,
    Return,
    True,
    False,
    Nil,
    And,
    Or,
    Not,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semicolon,

    Eof,
}

#[derive(Clone, Debug)]
pub struct SpannedToken {
    pub token: Token,
    pub line: u32,
}

/// Turns source text into a flat token stream. A hand-written scanner is
/// plenty for a language this small — no need for a generated lexer.
pub fn lex(source: &str) -> Result<Vec<SpannedToken>, ScriptError> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut line: u32 = 1;

    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\r' => i += 1,
            '\n' => {
                line += 1;
                i += 1;
            }
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '0'..='9' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                let value = text.parse::<f64>().map_err(|_| ScriptError {
                    message: format!("invalid number literal '{text}'"),
                    line,
                })?;
                tokens.push(SpannedToken {
                    token: Token::Number(value),
                    line,
                });
            }
            '"' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(ScriptError {
                        message: "unterminated string literal".to_string(),
                        line,
                    });
                }
                let text: String = chars[start..i].iter().collect();
                i += 1; // closing quote
                tokens.push(SpannedToken {
                    token: Token::Str(text),
                    line,
                });
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                let token = match text.as_str() {
                    "let" => Token::Let,
                    "fn" => Token::Fn,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "while" => Token::While,
                    "return" => Token::Return,
                    "true" => Token::True,
                    "false" => Token::False,
                    "nil" => Token::Nil,
                    "and" => Token::And,
                    "or" => Token::Or,
                    "not" => Token::Not,
                    _ => Token::Ident(text),
                };
                tokens.push(SpannedToken { token, line });
            }
            '+' => {
                tokens.push(SpannedToken {
                    token: Token::Plus,
                    line,
                });
                i += 1;
            }
            '-' => {
                tokens.push(SpannedToken {
                    token: Token::Minus,
                    line,
                });
                i += 1;
            }
            '*' => {
                tokens.push(SpannedToken {
                    token: Token::Star,
                    line,
                });
                i += 1;
            }
            '/' => {
                tokens.push(SpannedToken {
                    token: Token::Slash,
                    line,
                });
                i += 1;
            }
            '%' => {
                tokens.push(SpannedToken {
                    token: Token::Percent,
                    line,
                });
                i += 1;
            }
            '(' => {
                tokens.push(SpannedToken {
                    token: Token::LParen,
                    line,
                });
                i += 1;
            }
            ')' => {
                tokens.push(SpannedToken {
                    token: Token::RParen,
                    line,
                });
                i += 1;
            }
            '{' => {
                tokens.push(SpannedToken {
                    token: Token::LBrace,
                    line,
                });
                i += 1;
            }
            '}' => {
                tokens.push(SpannedToken {
                    token: Token::RBrace,
                    line,
                });
                i += 1;
            }
            ',' => {
                tokens.push(SpannedToken {
                    token: Token::Comma,
                    line,
                });
                i += 1;
            }
            ';' => {
                tokens.push(SpannedToken {
                    token: Token::Semicolon,
                    line,
                });
                i += 1;
            }
            '=' if chars.get(i + 1) == Some(&'=') => {
                tokens.push(SpannedToken {
                    token: Token::EqEq,
                    line,
                });
                i += 2;
            }
            '=' => {
                tokens.push(SpannedToken {
                    token: Token::Eq,
                    line,
                });
                i += 1;
            }
            '!' if chars.get(i + 1) == Some(&'=') => {
                tokens.push(SpannedToken {
                    token: Token::NotEq,
                    line,
                });
                i += 2;
            }
            '<' if chars.get(i + 1) == Some(&'=') => {
                tokens.push(SpannedToken {
                    token: Token::LtEq,
                    line,
                });
                i += 2;
            }
            '<' => {
                tokens.push(SpannedToken {
                    token: Token::Lt,
                    line,
                });
                i += 1;
            }
            '>' if chars.get(i + 1) == Some(&'=') => {
                tokens.push(SpannedToken {
                    token: Token::GtEq,
                    line,
                });
                i += 2;
            }
            '>' => {
                tokens.push(SpannedToken {
                    token: Token::Gt,
                    line,
                });
                i += 1;
            }
            other => {
                return Err(ScriptError {
                    message: format!("unexpected character '{other}'"),
                    line,
                });
            }
        }
    }

    tokens.push(SpannedToken {
        token: Token::Eof,
        line,
    });
    Ok(tokens)
}
