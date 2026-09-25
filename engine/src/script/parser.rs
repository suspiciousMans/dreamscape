use super::lexer::{lex, SpannedToken, Token};
use super::ScriptError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogicalOp {
    And,
    Or,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Number(f64),
    Str(String),
    Bool(bool),
    Nil,
    Ident(String),
    Unary(UnaryOp, Box<Expr>),
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Logical(Box<Expr>, LogicalOp, Box<Expr>),
    Assign(String, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Let(String, Expr),
    Expr(Expr),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    While(Expr, Vec<Stmt>),
    Return(Option<Expr>),
    FnDecl(FunctionDecl),
}

#[derive(Clone, Debug)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

/// Parses a full script into its top-level statements (a flat mix of
/// `let` bindings and `fn` declarations — no nested scripts/modules).
pub fn parse(source: &str) -> Result<Vec<Stmt>, Vec<ScriptError>> {
    let tokens = lex(source).map_err(|e| vec![e])?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        errors: Vec::new(),
    };
    let mut statements = Vec::new();
    while !parser.check(&Token::Eof) {
        match parser.statement() {
            Ok(stmt) => statements.push(stmt),
            Err(err) => {
                parser.errors.push(err);
                parser.synchronize();
            }
        }
    }
    if parser.errors.is_empty() {
        Ok(statements)
    } else {
        Err(parser.errors)
    }
}

struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    errors: Vec<ScriptError>,
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos].token
    }

    fn line(&self) -> u32 {
        self.tokens[self.pos].line
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].token.clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, token: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(token)
    }

    fn matches(&mut self, token: &Token) -> bool {
        if self.check(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: Token, context: &str) -> Result<(), ScriptError> {
        if self.check(&token) {
            self.advance();
            Ok(())
        } else {
            Err(ScriptError {
                message: format!("expected {context}, found {:?}", self.peek()),
                line: self.line(),
            })
        }
    }

    /// After a parse error, skip tokens until a likely statement boundary so
    /// a single typo doesn't cascade into dozens of spurious errors.
    fn synchronize(&mut self) {
        while !self.check(&Token::Eof) {
            if matches!(self.peek(), Token::Semicolon) {
                self.advance();
                return;
            }
            if matches!(
                self.peek(),
                Token::Fn | Token::Let | Token::If | Token::While | Token::Return
            ) {
                return;
            }
            self.advance();
        }
    }

    fn statement(&mut self) -> Result<Stmt, ScriptError> {
        if self.matches(&Token::Let) {
            return self.let_statement();
        }
        if self.matches(&Token::Fn) {
            return self.fn_decl();
        }
        if self.matches(&Token::If) {
            return self.if_statement();
        }
        if self.matches(&Token::While) {
            return self.while_statement();
        }
        if self.matches(&Token::Return) {
            let value = if self.check(&Token::Semicolon) {
                None
            } else {
                Some(self.expr()?)
            };
            self.expect(Token::Semicolon, "';' after return")?;
            return Ok(Stmt::Return(value));
        }
        let expr = self.expr()?;
        self.expect(Token::Semicolon, "';' after expression")?;
        Ok(Stmt::Expr(expr))
    }

    fn let_statement(&mut self) -> Result<Stmt, ScriptError> {
        let name = self.ident("variable name after 'let'")?;
        self.expect(Token::Eq, "'=' after variable name")?;
        let value = self.expr()?;
        self.expect(Token::Semicolon, "';' after 'let' initializer")?;
        Ok(Stmt::Let(name, value))
    }

    fn fn_decl(&mut self) -> Result<Stmt, ScriptError> {
        let name = self.ident("function name after 'fn'")?;
        self.expect(Token::LParen, "'(' after function name")?;
        let mut params = Vec::new();
        if !self.check(&Token::RParen) {
            loop {
                params.push(self.ident("parameter name")?);
                if !self.matches(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(Token::RParen, "')' after parameters")?;
        let body = self.block()?;
        Ok(Stmt::FnDecl(FunctionDecl { name, params, body }))
    }

    fn if_statement(&mut self) -> Result<Stmt, ScriptError> {
        let condition = self.expr()?;
        let then_branch = self.block()?;
        let else_branch = if self.matches(&Token::Else) {
            if self.matches(&Token::If) {
                Some(vec![self.if_statement()?])
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        Ok(Stmt::If(condition, then_branch, else_branch))
    }

    fn while_statement(&mut self) -> Result<Stmt, ScriptError> {
        let condition = self.expr()?;
        let body = self.block()?;
        Ok(Stmt::While(condition, body))
    }

    fn block(&mut self) -> Result<Vec<Stmt>, ScriptError> {
        self.expect(Token::LBrace, "'{'")?;
        let mut statements = Vec::new();
        while !self.check(&Token::RBrace) && !self.check(&Token::Eof) {
            statements.push(self.statement()?);
        }
        self.expect(Token::RBrace, "'}'")?;
        Ok(statements)
    }

    fn ident(&mut self, context: &str) -> Result<String, ScriptError> {
        match self.advance() {
            Token::Ident(name) => Ok(name),
            other => Err(ScriptError {
                message: format!("expected {context}, found {other:?}"),
                line: self.line(),
            }),
        }
    }

    // --- expressions, lowest to highest precedence ---

    fn expr(&mut self) -> Result<Expr, ScriptError> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, ScriptError> {
        let expr = self.or_expr()?;
        if self.matches(&Token::Eq) {
            let value = self.assignment()?;
            if let Expr::Ident(name) = expr {
                return Ok(Expr::Assign(name, Box::new(value)));
            }
            return Err(ScriptError {
                message: "invalid assignment target".to_string(),
                line: self.line(),
            });
        }
        Ok(expr)
    }

    fn or_expr(&mut self) -> Result<Expr, ScriptError> {
        let mut expr = self.and_expr()?;
        while self.matches(&Token::Or) {
            let rhs = self.and_expr()?;
            expr = Expr::Logical(Box::new(expr), LogicalOp::Or, Box::new(rhs));
        }
        Ok(expr)
    }

    fn and_expr(&mut self) -> Result<Expr, ScriptError> {
        let mut expr = self.equality()?;
        while self.matches(&Token::And) {
            let rhs = self.equality()?;
            expr = Expr::Logical(Box::new(expr), LogicalOp::And, Box::new(rhs));
        }
        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr, ScriptError> {
        let mut expr = self.comparison()?;
        loop {
            let op = if self.matches(&Token::EqEq) {
                BinaryOp::Eq
            } else if self.matches(&Token::NotEq) {
                BinaryOp::NotEq
            } else {
                break;
            };
            let rhs = self.comparison()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr, ScriptError> {
        let mut expr = self.term()?;
        loop {
            let op = if self.matches(&Token::Lt) {
                BinaryOp::Lt
            } else if self.matches(&Token::LtEq) {
                BinaryOp::LtEq
            } else if self.matches(&Token::Gt) {
                BinaryOp::Gt
            } else if self.matches(&Token::GtEq) {
                BinaryOp::GtEq
            } else {
                break;
            };
            let rhs = self.term()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, ScriptError> {
        let mut expr = self.factor()?;
        loop {
            let op = if self.matches(&Token::Plus) {
                BinaryOp::Add
            } else if self.matches(&Token::Minus) {
                BinaryOp::Sub
            } else {
                break;
            };
            let rhs = self.factor()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr, ScriptError> {
        let mut expr = self.unary()?;
        loop {
            let op = if self.matches(&Token::Star) {
                BinaryOp::Mul
            } else if self.matches(&Token::Slash) {
                BinaryOp::Div
            } else if self.matches(&Token::Percent) {
                BinaryOp::Mod
            } else {
                break;
            };
            let rhs = self.unary()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, ScriptError> {
        if self.matches(&Token::Minus) {
            let expr = self.unary()?;
            return Ok(Expr::Unary(UnaryOp::Neg, Box::new(expr)));
        }
        if self.matches(&Token::Not) {
            let expr = self.unary()?;
            return Ok(Expr::Unary(UnaryOp::Not, Box::new(expr)));
        }
        self.call()
    }

    fn call(&mut self) -> Result<Expr, ScriptError> {
        let primary = self.primary()?;
        if let Expr::Ident(name) = &primary {
            if self.matches(&Token::LParen) {
                let mut args = Vec::new();
                if !self.check(&Token::RParen) {
                    loop {
                        args.push(self.expr()?);
                        if !self.matches(&Token::Comma) {
                            break;
                        }
                    }
                }
                self.expect(Token::RParen, "')' after arguments")?;
                return Ok(Expr::Call(name.clone(), args));
            }
        }
        Ok(primary)
    }

    fn primary(&mut self) -> Result<Expr, ScriptError> {
        let line = self.line();
        match self.advance() {
            Token::Number(n) => Ok(Expr::Number(n)),
            Token::Str(s) => Ok(Expr::Str(s)),
            Token::True => Ok(Expr::Bool(true)),
            Token::False => Ok(Expr::Bool(false)),
            Token::Nil => Ok(Expr::Nil),
            Token::Ident(name) => Ok(Expr::Ident(name)),
            Token::LParen => {
                let expr = self.expr()?;
                self.expect(Token::RParen, "')' after expression")?;
                Ok(expr)
            }
            other => Err(ScriptError {
                message: format!("unexpected token {other:?}"),
                line,
            }),
        }
    }
}
