use crate::ast::Delimiter;
use crate::{
    ast::{BindingMode, Local, LocalKind, Mutability, Stmt, StmtKind, TokenKind},
    kw::Keyword,
};

// use crate::interner::Interner;
use super::{PResult, Parser};

// block_statement = { statement* }
// statement = block_statement
//             | declaration_statement
//             | expression_statement
//             | if_statement
//             | loop_statement
//
//
// loop_statement = predicate_loop_statement
//                 | iterator_loop_statement
//
// predicate_loop_statement = 'while' expression block_statement
// iterator_loop_statement = 'for' identifier 'in' expression block_statement
//
//
// expression_statement = expression ';'
// expression = assignment_expression
//              | binary_expression
//              | unary_expression
//              | literal
//
//
// declaration_statement = declaration
// declaration = variable_declaration
//               | function_declaration
//               | class_declaration
//               | enum_declaration
//               | interface_declaration
//
//
// variable_declaration = 'var' 'mut'? identifier: type_specifier ('=' expression)? ';'
// type_specifier = path
// path = segment ('::' segment)*
// segment = identifier ('<' generic_args '>')?
// generic_args = angle_bracketed_args
// angle_bracketed_args = '<' angle_bracketed_arg (',' angle_bracketed_arg)* '>'
// angle_bracketed_arg = generic_arg
// generic_arg = type_specifier

impl Parser<'_> {
    pub fn parse_stmt(&mut self) -> PResult<Box<Stmt>> {
        if self.token.is_keyword(Keyword::Var) {
            self.parse_stmt_var_decl()
        } else if self.token.is_keyword(Keyword::If) {
            self.parse_stmt_if()
        } else if self.token.can_begin_expr() {
            self.parse_stmt_expr()
        } else if self.token.kind == TokenKind::OpenDelim(Delimiter::Brace) {
            self.parse_stmt_block()
        } else if self.token.is_keyword(Keyword::While) {
            self.parse_stmt_while()
        } else if self.token.is_keyword(Keyword::For) {
            self.parse_stmt_for()
        } else if self.token.is_keyword(Keyword::Return) {
            self.parse_stmt_return()
        } else {
            unreachable!();
        }
    }

    pub fn parse_stmt_return(&mut self) -> PResult<Box<Stmt>>{
        let start_span = self.token.span;
        let kind = if self.look_ahead(1,|tok|tok.can_begin_expr()){
            self.advance(); // Eat token after "return"
            let expr = self.parse_expr()?;
            self.advance(); // Eat token after ';'
            StmtKind::Return(Some(expr))
        } else if self.look_ahead(1,|tok|tok.kind == TokenKind::Semicolon) {
            self.advance(); // Eat token after "return" <=> ';'
            self.advance(); // Eat token after ';'
            StmtKind::Return(None)
        } else {
            return Err("Expected an expr or ';'".into())
        };
        self.pass_semicolon();
        Ok(Box::new(
            Stmt{
                kind,
                span:start_span.to(self.prev_token.span),
            }
        ))
    }

    /// predicate_loop_statement = 'while' expression block_statement
    pub fn parse_stmt_while(&mut self) -> PResult<Box<Stmt>> {
        if !self.token.is_keyword(Keyword::While) {
            return Err("Expected 'while'".into());
        }

        let start_span = self.token.span;
        self.advance();

        let condition = self.parse_expr()?;
        let block = self.parse_stmt_block()?;
        let end_span = self.prev_token.span;
        let span = start_span.to(end_span);
        let kind = StmtKind::While(condition, block);
        let stmt = Box::new(Stmt { kind, span });

        self.pass_semicolon();

        Ok(stmt)
    }

    /// iterator_loop_statement = 'for' identifier 'in' expression block_statement
    pub fn parse_stmt_for(&mut self) -> PResult<Box<Stmt>> {
        if !self.token.is_keyword(Keyword::For) {
            return Err("Expected 'for'".into());
        }

        let start_span = self.token.span;

        self.advance();
        if !self.token.is_ident() {
            return Err("Expected identifier".into());
        }
        let ident = self.token.ident().unwrap().0;

        self.advance();
        if !self.token.is_keyword(Keyword::In) {
            return Err("Expected 'in'".into());
        }

        self.advance();
        let expr = self.parse_expr()?;
        let block = self.parse_stmt_block()?;
        let end_span = self.prev_token.span;
        let span = start_span.to(end_span);
        let kind = StmtKind::For(ident, expr, block);
        let stmt = Box::new(Stmt { kind, span });

        self.pass_semicolon();

        Ok(stmt)
    }

    /// if_statement = 'if' expression block_statement ('else' (block_statement | if_statement))?
    pub fn parse_stmt_if(&mut self) -> PResult<Box<Stmt>> {
        if !self.token.is_keyword(Keyword::If) {
            return Err(format!("Expected 'if', found {:?}", self.token).into());
        }

        let start_span = self.token.span;

        self.advance(); // Eat token after "if"
                        // Parse the condition expression.
        let condition = self.parse_expr()?;
        // Parse the block for the `if` statement.
        let if_block = self.parse_stmt_block()?;

        // Optionally parse an `else` block.
        let else_block = if self.token.is_keyword(Keyword::Else) {
            self.advance(); // Eat token after `else`
            if self.token.is_keyword(Keyword::If) {
                let else_block = self.parse_stmt_if()?;
                Some(else_block)
            } else if self.token.kind == TokenKind::OpenDelim(Delimiter::Brace) {
                let else_block = self.parse_stmt_block()?;
                Some(else_block)
            } else {
                return Err("Expected 'if' or '{'".into());
            }
        } else {
            None
        };

        let end_span = self.prev_token.span;
        let span = start_span.to(end_span);
        let kind = StmtKind::If(condition, if_block, else_block);
        let stmt = Box::new(Stmt { kind, span });

        self.pass_semicolon();

        Ok(stmt)
    }

    /// block_statement = '{' statement* '}'
    pub fn parse_stmt_block(&mut self) -> PResult<Box<Stmt>> {
        if self.token.kind != TokenKind::OpenDelim(Delimiter::Brace) {
            return Err(format!("Expected '{{' in block statement, found {:?}", self.token).into());
        }

        let start = self.token.span;
        self.advance(); // Eat token after '{'

        let mut stmts = Vec::new();
        while self.token.kind != TokenKind::CloseDelim(Delimiter::Brace) {
            let stmt = self.parse_stmt()?;
            stmts.push(stmt);
        }

        let end = self.token.span;
        let span = start.to(end);
        let kind = StmtKind::Block(stmts);
        let stmt = Box::new(Stmt { kind, span });

        self.advance(); // Eat token after '}'

        self.pass_semicolon();

        Ok(stmt)
    }

    /// expression_statement = expression ';'
    pub fn parse_stmt_expr(&mut self) -> PResult<Box<Stmt>> {
        let expr = self.parse_expr()?;
        let span = expr.span;
        let stmt = Box::new(Stmt {
            kind: StmtKind::Expr(expr),
            span,
        });

        if self.token.kind != TokenKind::Semicolon {
            return Err("Expected ';'".into());
        }
        self.advance(); // Eat token after ';'

        self.pass_semicolon();

        Ok(stmt)
    }

    /// variable_declaration = 'var' 'mut'? identifier: type_specifier ('=' expression)? ';'
    pub fn parse_stmt_var_decl(&mut self) -> PResult<Box<Stmt>> {
        if !self.token.is_keyword(Keyword::Var) {
            return Err("Expected 'var'".into());
        }

        let start = self.token.span;
        let binding_mode = if self.is_keyword_ahead(&[Keyword::Mut]) {
            self.advance(); // 'mut'
            BindingMode(Mutability::Mutable)
        } else {
            BindingMode(Mutability::Immutable)
        };

        if !self.is_ident_ahead() {
            return Err("Expected identifier".into());
        }
        self.advance(); // identifier
        let ident = self.token.ident().unwrap().0;

        if !self.look_ahead(1, |tok| tok.kind == TokenKind::Colon) {
            return Err("Expected ':'".into());
        }
        self.advance(); // ':'

        self.advance(); // type
        let ty = self.parse_ty()?;

        let init = if self.token.kind == TokenKind::Eq {
            self.advance(); // expr
            Some(self.parse_expr()?)
        } else {
            None
        };

        if self.token.kind != TokenKind::Semicolon {
            return Err("Expected ';'".into());
        }
        let kind = if let Some(init) = init {
            LocalKind::Init(init)
        } else {
            LocalKind::Decl
        };
        let span = start.to(self.token.span);
        let local = Local {
            binding_mode,
            ident,
            ty,
            kind,
            span,
        };
        let kind = StmtKind::Var(Box::new(local));
        let stmt = Box::new(Stmt { kind, span });

        self.advance();

        self.pass_semicolon();

        Ok(stmt)
    }

    pub fn pass_semicolon(&mut self) {
        while self.token.kind == TokenKind::Semicolon {
            self.advance();
        }
    }
}
