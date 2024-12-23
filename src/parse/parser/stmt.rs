use super::{PResult, Parser, TokenType};
use crate::ast::{Delimiter, Fun, FunParam, FunSig};
use crate::kw;
use crate::parse::errors::PError;
use crate::{
    ast::{Local, LocalKind, Stmt, StmtKind, TokenKind},
    kw::Keyword,
};

impl Parser {
    // Remember to update ast::token::can_begin_expr as well when adding new statements.
    pub fn parse_stmt(&mut self) -> PResult<Box<Stmt>> {
        if self.token.is_keyword(Keyword::Set) {
            self.parse_stmt_var_decl()
        } else if self.token.is_keyword(Keyword::When) {
            self.parse_stmt_if()
        } else if self.token.kind == TokenKind::OpenDelim(Delimiter::Brace) {
            self.parse_stmt_block()
        } else if self.token.is_keyword(Keyword::During) {
            self.parse_stmt_while()
        } else if self.token.is_keyword(Keyword::For) {
            self.parse_stmt_for()
        } else if self.token.is_keyword(Keyword::Yeet) {
            self.parse_stmt_return()
        } else if self.token.kind == TokenKind::Semicolon {
            self.parse_stmt_empty()
        } else if self.token.is_keyword(Keyword::Fun) {
            self.parse_stmt_func_decl()
        } else if self.token.is_keyword(Keyword::Add) {
            self.parse_stmt_import()
        } else if self.token.is_keyword(Keyword::Br) {
            self.parse_stmt_break()
        } else if self.token.is_keyword(Keyword::Skip) {
            self.parse_stmt_continue()
        } else if self.token.can_begin_expr() {
            self.parse_stmt_expr()
        } else {
            let err = PError::ExpectedStatement {
                token: TokenType::Token(self.token.kind),
                span: self.token.span,
            };

            return Err(vec![err]);
        }
    }

    fn parse_stmt_continue(&mut self) -> PResult<Box<Stmt>> {
        if !self.token.is_keyword(Keyword::Skip) {
            let err = PError::ExpectedToken {
                expected: vec![TokenType::Keyword(kw::to_symbol(Keyword::Skip))],
                found: TokenType::Token(self.token.kind),
                span: self.token.span,
                prev_span: self.prev_token.span,
            };

            return Err(vec![err]);
        }

        let span = self.token.span;
        self.advance(); // Eat token after "skip"

        self.expect(TokenKind::Semicolon)?;
        let span = span.to(self.token.span);
        self.advance();

        Ok(Box::new(Stmt {
            kind: StmtKind::Continue,
            span,
        }))
    }

    fn parse_stmt_break(&mut self) -> PResult<Box<Stmt>> {
        if !self.token.is_keyword(Keyword::Br) {
            let err = PError::ExpectedToken {
                expected: vec![TokenType::Keyword(kw::to_symbol(Keyword::Br))],
                found: TokenType::Token(self.token.kind),
                span: self.token.span,
                prev_span: self.prev_token.span,
            };

            return Err(vec![err]);
        }

        let span = self.token.span;
        self.advance(); // Eat token after "exit"

        self.expect(TokenKind::Semicolon)?;
        let span = span.to(self.token.span);
        self.advance();

        Ok(Box::new(Stmt {
            kind: StmtKind::Break,
            span,
        }))
    }

    fn parse_stmt_import(&mut self) -> PResult<Box<Stmt>> {
        if !self.token.is_keyword(Keyword::Add) {
            let err = PError::ExpectedToken {
                expected: vec![TokenType::Keyword(kw::to_symbol(Keyword::Add))],
                found: TokenType::Token(self.token.kind),
                span: self.token.span,
                prev_span: self.prev_token.span,
            };

            return Err(vec![err]);
        }

        let start_span = self.token.span;
        self.advance(); // Eat "import"

        let path = self.parse_ident()?;
        self.expect(TokenKind::Semicolon)?;
        let span = start_span.to(self.token.span);
        self.advance();

        let kind = StmtKind::Import(path);
        let stmt = Box::new(Stmt { kind, span });

        Ok(stmt)
    }

    fn parse_stmt_func_decl(&mut self) -> PResult<Box<Stmt>> {
        if !self.token.is_keyword(Keyword::Fun) {
            let err = PError::ExpectedToken {
                expected: vec![TokenType::Keyword(kw::to_symbol(Keyword::Fun))],
                found: TokenType::Token(self.token.kind),
                span: self.token.span,
                prev_span: self.prev_token.span,
            };

            return Err(vec![err]);
        }
        let start_span = self.token.span;
        self.advance(); // Eat "fn"

        let sig = self.parse_stmt_func_sig()?;
        let body = self.parse_stmt()?;

        let end_span = self.prev_token.span;
        let span = start_span.to(end_span);
        let kind = StmtKind::FuncDecl(Box::new(Fun { sig, body }));
        let stmt = Box::new(Stmt { kind, span });

        Ok(stmt)
    }

    fn parse_stmt_return(&mut self) -> PResult<Box<Stmt>> {
        if !self.token.is_keyword(Keyword::Yeet) {
            let err = PError::ExpectedToken {
                expected: vec![TokenType::Keyword(kw::to_symbol(Keyword::Yeet))],
                found: TokenType::Token(self.token.kind),
                span: self.token.span,
                prev_span: self.prev_token.span,
            };

            return Err(vec![err]);
        }

        let start_span = self.token.span;
        self.advance();
        let kind = if self.token.can_begin_expr() {
            let expr = self.parse_expr()?;
            self.expect(TokenKind::Semicolon)?;
            self.advance();
            StmtKind::Return(Some(expr))
        } else {
            self.expect(TokenKind::Semicolon)?;
            self.advance();
            StmtKind::Return(None)
        };

        Ok(Box::new(Stmt {
            kind,
            span: start_span.to(self.prev_token.span),
        }))
    }

    /// predicate_loop_statement = 'while' expression block_statement
    fn parse_stmt_while(&mut self) -> PResult<Box<Stmt>> {
        if !self.token.is_keyword(Keyword::During) {
            let err = PError::ExpectedToken {
                expected: vec![TokenType::Keyword(kw::to_symbol(Keyword::During))],
                found: TokenType::Token(self.token.kind),
                span: self.token.span,
                prev_span: self.prev_token.span,
            };

            return Err(vec![err]);
        }

        let start_span = self.token.span;
        self.advance();

        let condition = self.parse_expr()?;
        let block = self.parse_stmt()?;
        let end_span = self.prev_token.span;
        let span = start_span.to(end_span);
        let kind = StmtKind::While(condition, block);
        let stmt = Box::new(Stmt { kind, span });

        Ok(stmt)
    }

    /// iterator_loop_statement = 'for' identifier 'in' expression block_statement
    fn parse_stmt_for(&mut self) -> PResult<Box<Stmt>> {
        if !self.token.is_keyword(Keyword::For) {
            let err = PError::ExpectedToken {
                expected: vec![TokenType::Keyword(kw::to_symbol(Keyword::For))],
                found: TokenType::Token(self.token.kind),
                span: self.token.span,
                prev_span: self.prev_token.span,
            };

            return Err(vec![err]);
        }

        let start_span = self.token.span;

        self.advance();
        let ident = self.parse_ident()?;

        if !self.token.is_keyword(Keyword::In) {
            let err = PError::ExpectedToken {
                expected: vec![TokenType::Keyword(kw::to_symbol(Keyword::In))],
                found: TokenType::Token(self.token.kind),
                span: self.token.span,
                prev_span: self.prev_token.span,
            };

            return Err(vec![err]);
        }

        self.advance();
        let expr = self.parse_expr()?;
        let block = self.parse_stmt()?;
        let end_span = self.prev_token.span;
        let span = start_span.to(end_span);
        let kind = StmtKind::For(ident, expr, block);
        let stmt = Box::new(Stmt { kind, span });

        Ok(stmt)
    }

    /// if_statement = 'if' expression block_statement ('else' (block_statement | if_statement))?
    fn parse_stmt_if(&mut self) -> PResult<Box<Stmt>> {
        if !self.token.is_keyword(Keyword::When) {
            let err = PError::ExpectedToken {
                expected: vec![TokenType::Keyword(kw::to_symbol(Keyword::When))],
                found: TokenType::Token(self.token.kind),
                span: self.token.span,
                prev_span: self.prev_token.span,
            };

            return Err(vec![err]);
        }

        let start_span = self.token.span;
        self.advance(); // Eat token after "if"
                        // Parse the condition expression.
        let condition = self.parse_expr()?;
        // Parse the block for the `if` statement.
        let if_block = self.parse_stmt()?;

        // Optionally parse an `else` block.
        let else_block = if self.token.is_keyword(Keyword::Alt) {
            self.advance(); // Eat token after `else`
            let else_block = self.parse_stmt()?;
            Some(else_block)
        } else {
            None
        };

        let end_span = self.prev_token.span;
        let span = start_span.to(end_span);
        let kind = StmtKind::If(condition, if_block, else_block);
        let stmt = Box::new(Stmt { kind, span });

        Ok(stmt)
    }

    /// block_statement = '{' statement* '}'
    pub fn parse_stmt_block(&mut self) -> PResult<Box<Stmt>> {
        self.expect(TokenKind::OpenDelim(Delimiter::Brace))?;
        let start = self.token.span;
        self.advance();

        let mut stmts = Vec::new();
        let mut errors: Vec<PError> = Vec::new();
        // Parse statements until we reach the end of the block.
        while self.token.kind != TokenKind::CloseDelim(Delimiter::Brace) {
            let result = self.parse_stmt();

            if let Err(mut err) = result {
                errors.append(&mut err);
                self.recover();

                continue;
            }

            let stmt = result.unwrap();
            stmts.push(stmt);
        }

        let end = self.token.span;
        let span = start.to(end);
        let kind = StmtKind::Block(stmts);
        let stmt = Box::new(Stmt { kind, span });

        // This must be true because we have already checked for the closing brace.
        self.expect(TokenKind::CloseDelim(Delimiter::Brace))
            .unwrap();
        self.advance(); // Eat '}'

        if errors.is_empty() {
            Ok(stmt)
        } else {
            Err(errors)
        }
    }

    /// expression_statement = expression ';'
    pub fn parse_stmt_expr(&mut self) -> PResult<Box<Stmt>> {
        let expr = self.parse_expr()?;
        let span = expr.span;
        let stmt = Box::new(Stmt {
            kind: StmtKind::Expr(expr),
            span,
        });

        self.expect(TokenKind::Semicolon)?;
        self.advance();

        Ok(stmt)
    }

    /// variable_declaration = 'var' 'mut'? identifier: type_specifier ('=' expression)? ';'
    fn parse_stmt_var_decl(&mut self) -> PResult<Box<Stmt>> {
        if !self.token.is_keyword(Keyword::Set) {
            let err = PError::ExpectedToken {
                expected: vec![TokenType::Keyword(kw::to_symbol(Keyword::Set))],
                found: TokenType::Token(self.token.kind),
                span: self.token.span,
                prev_span: self.prev_token.span,
            };

            return Err(vec![err]);
        }

        let start = self.token.span;
        self.advance(); // 'set'

        let is_mut = if self.token.is_keyword(Keyword::Mut) {
            self.advance(); // 'mut'
            true
        } else {
            false
        };

        let ident = self.parse_ident()?;
        self.expect(TokenKind::Colon)?;
        self.advance(); // ':'
        let ty = self.parse_ty()?;

        let init = if self.token.kind == TokenKind::Eq {
            self.advance(); // expr
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect(TokenKind::Semicolon)?;
        let span = start.to(self.token.span);

        self.advance();

        let kind = if let Some(init) = init {
            LocalKind::Init(init)
        } else {
            LocalKind::Decl
        };

        let local = Local {
            is_mut,
            ident,
            ty,
            kind,
            span,
        };
        let kind = StmtKind::Var(Box::new(local));
        let stmt = Box::new(Stmt { kind, span });

        Ok(stmt)
    }

    pub fn parse_stmt_empty(&mut self) -> PResult<Box<Stmt>> {
        self.expect(TokenKind::Semicolon)?;

        let span = self.token.span;
        self.advance();
        Ok(Box::new(Stmt {
            kind: StmtKind::Empty,
            span,
        }))
    }

    fn parse_stmt_func_sig(&mut self) -> PResult<FunSig> {
        let start = self.token.span;
        let name = self.parse_ident()?;

        self.expect(TokenKind::OpenDelim(Delimiter::Parenthesis))?;
        self.advance();

        let mut inputs: Vec<FunParam> = Vec::new();
        loop {
            if self.token.is_close_delim(Delimiter::Parenthesis) {
                break;
            }

            let start = self.token.span;

            let is_mut = if self.token.is_keyword(Keyword::Mut) {
                self.advance(); // Eat 'mut'
                true
            } else {
                false
            };

            let ident = self.parse_ident()?;

            self.expect(TokenKind::Colon)?;
            self.advance(); // Eat ':'

            let ty = self.parse_ty()?;
            let end = self.prev_token.span;
            inputs.push(FunParam {
                ident,
                ty,
                is_mut,
                span: start.to(end),
            });

            if self.token.kind != TokenKind::Comma {
                break;
            }

            self.advance(); // Eat ','
        }

        self.expect(TokenKind::CloseDelim(Delimiter::Parenthesis))?;
        self.advance(); // Eat ')'

        let output = if self.token.kind == TokenKind::RArrow {
            self.advance(); // Eat '->'
            Some(self.parse_ty()?)
        } else {
            None
        };

        let end = self.prev_token.span;
        let span = start.to(end);

        Ok(FunSig {
            name,
            inputs,
            output,
            span,
        })
    }
}
