use crate::{
    ast::{
        BinOp, BinOpKind, BinOpToken, Delimiter, Expr, ExprKind, Lit, LitKind, TokenKind, Ty, UnOp,
    },
    parse::util::parser::{AssocOp, Fixity},
    span_encoding::Span,
};

use super::{PResult, Parser, TokenType};
use crate::span_encoding;

impl Parser<'_> {
    pub fn parse_expr(&mut self) -> PResult<Box<Expr>> {
        let lhs = self.parse_expr_prefix()?;
        self.parse_expr_rest(0, lhs)
    }

    fn parse_expr_rest(&mut self, min_prec: usize, mut lhs: Box<Expr>) -> PResult<Box<Expr>> {
        self.expected_tokens.push(TokenType::Operator);

        loop {
            // TODO: Handle error probably later.
            let op_assoc = AssocOp::from_token(&self.token);
            if op_assoc.is_none() {
                break;
            }
            let op_assoc = op_assoc.unwrap();
            let prec = op_assoc.precedence();
            if prec < min_prec {
                break;
            }

            let lhs_span = lhs.span;

            // Special cases:
            if op_assoc == AssocOp::As {
                lhs = self.parse_assoc_op_cast(lhs, lhs_span, ExprKind::Cast)?;
                continue;
            }

            let op_span = self.token.span;
            self.advance();

            let mut rhs = self.parse_expr_prefix()?;
            let fixity = op_assoc.fixity();
            let next_prec = match fixity {
                Fixity::Left => prec + 1,
                Fixity::Right => prec,
            };
            rhs = self.parse_expr_rest(next_prec, rhs)?;
            let span = self.mk_expr_sp(&lhs, rhs.span);
            lhs = match op_assoc {
                AssocOp::Add
                | AssocOp::Subtract
                | AssocOp::Multiply
                | AssocOp::Divide
                | AssocOp::Modulus
                | AssocOp::LAnd
                | AssocOp::LOr
                | AssocOp::BitXor
                | AssocOp::BitAnd
                | AssocOp::BitOr
                | AssocOp::ShiftLeft
                | AssocOp::ShiftRight
                | AssocOp::Equal
                | AssocOp::Less
                | AssocOp::LessEqual
                | AssocOp::NotEqual
                | AssocOp::Greater
                | AssocOp::GreaterEqual => {
                    let ast_op = op_assoc.to_ast_binop().unwrap();
                    let binary = self.mk_binary(span_encoding::respan(ast_op, op_span), lhs, rhs);
                    self.mk_expr(binary, span)
                }
                AssocOp::Assign => self.mk_expr(ExprKind::Assign(lhs, rhs, op_span), span),
                AssocOp::AssignOp(k) => {
                    let aop = match k {
                        BinOpToken::Plus => BinOpKind::Add,
                        BinOpToken::Minus => BinOpKind::Sub,
                        BinOpToken::Star => BinOpKind::Mul,
                        BinOpToken::Slash => BinOpKind::Div,
                        BinOpToken::Percent => BinOpKind::Mod,
                        BinOpToken::Caret => BinOpKind::BitXor,
                        BinOpToken::And => BinOpKind::BitAnd,
                        BinOpToken::Or => BinOpKind::BitOr,
                        BinOpToken::Shl => BinOpKind::Shl,
                        BinOpToken::Shr => BinOpKind::Shr,
                    };
                    let aopexpr = self.mk_assign_op(span_encoding::respan(aop, op_span), lhs, rhs);
                    self.mk_expr(aopexpr, span)
                }
                AssocOp::As => unreachable!("AssocOp::As should be handled separately"),
            }
        }

        Ok(lhs)
    }

    fn parse_assoc_op_cast(
        &mut self,
        lhs: Box<Expr>,
        _lhs_span: Span,
        expr_kind: fn(Box<Expr>, Ty) -> ExprKind,
    ) -> PResult<Box<Expr>> {
        self.advance(); // eat 'as'
        let ty = self.parse_ty()?;
        let span = self.mk_expr_sp(&lhs, ty.span);
        let cast = expr_kind(lhs, ty);
        Ok(self.mk_expr(cast, span))
    }

    /// Parses a prefix-unary-operator expr.
    fn parse_expr_prefix(&mut self) -> PResult<Box<Expr>> {
        match self.token.kind {
            TokenKind::Not => {
                self.advance();
                let expr = self.parse_expr_prefix()?;
                let span = expr.span;
                let expr = self.mk_unary(UnOp::Not, expr);
                Ok(self.mk_expr(expr, span))
            }
            TokenKind::BinOp(BinOpToken::Minus) => {
                self.advance();
                let expr = self.parse_expr_prefix()?;
                let span = self.token.span;
                let expr = self.mk_unary(UnOp::Ne, expr);
                Ok(self.mk_expr(expr, span))
            }
            _ => self.parse_expr_call(),
        }
    }

    /// Parses a dot or call expression.
    /// DotOrCall = Expr '.' Ident | Expr '(' [Expr] ')'
    fn parse_expr_call(&mut self) -> PResult<Box<Expr>> {
        let base = self.parse_expr_bottom()?;
        if self.token.is_open_delim(Delimiter::Parenthesis) {
            self.parse_expr_call_with(base)
        } else {
            Ok(base)
        }
    }

    /// Parses a call expression.
    /// Call = Expr '(' [Expr] ')'
    fn parse_expr_call_with(&mut self, base: Box<Expr>) -> PResult<Box<Expr>> {
        debug_assert!(self.token.is_open_delim(Delimiter::Parenthesis));
        self.advance();

        let mut args = Vec::new();
        loop {
            if self.token.is_close_delim(Delimiter::Parenthesis) {
                break;
            }

            let arg = self.parse_expr()?;
            args.push(arg);

            if !self.token.is_kind(TokenKind::Comma) {
                break;
            }

            self.advance(); // eat comma
        }

        if !self.token.is_close_delim(Delimiter::Parenthesis) {
            return Err("Expected ')'".into());
        }

        let span = self.mk_expr_sp(&base, self.token.span);
        let call = ExprKind::FunCall(base, args);
        self.advance();

        Ok(self.mk_expr(call, span))
    }

    /// Highest precedence level.
    fn parse_expr_bottom(&mut self) -> PResult<Box<Expr>> {
        match self.token.kind {
            TokenKind::Literal(_) => self.parse_expr_lit(),
            TokenKind::Ident(_, _) => self.parse_expr_ident(),
            TokenKind::OpenDelim(Delimiter::Parenthesis) => {
                self.parse_expr_grouped(Delimiter::Parenthesis)
            }
            _ => Err(format!("Unexpected token: {:?}", self.token).into()),
        }
    }

    fn parse_expr_ident(&mut self) -> PResult<Box<Expr>> {
        debug_assert!(self.token.is_ident());
        if self.look_ahead(0, |tok| tok.ident().unwrap().0.name.is_bool_lit()) {
            self.parse_expr_lit()
        } else {
            let ident = self.parse_ident()?;
            let span = ident.span;
            let expr = ExprKind::Identifier(ident);
            Ok(self.mk_expr(expr, span))
        }
    }

    fn parse_expr_grouped(&mut self, delim: Delimiter) -> PResult<Box<Expr>> {
        self.advance();
        let expr = self.parse_expr()?;
        self.expect(TokenKind::CloseDelim(delim))?;
        self.advance();
        Ok(expr)
    }

    /// Parses a literal expression. Lit = true | false | token_lit~
    fn parse_expr_lit(&mut self) -> PResult<Box<Expr>> {
        let lit = self.parse_token_lit()?;
        let span = self.prev_token.span;
        let expr = self.mk_literal(lit);
        Ok(self.mk_expr(expr, span))
    }

    fn parse_token_lit(&mut self) -> PResult<Lit> {
        match self.token.kind {
            TokenKind::Literal(lit) => {
                self.advance();
                Ok(lit)
            }
            TokenKind::Ident(sym, _) => {
                if sym.is_bool_lit() {
                    let kind = LitKind::Bool;
                    let symbol = sym;
                    self.advance();
                    Ok(Lit { kind, symbol })
                } else {
                    Err("Expected literal".into())
                }
            }
            _ => Err("Expected literal".into()),
        }
    }

    fn mk_literal(&self, lit: Lit) -> ExprKind {
        ExprKind::Literal(lit)
    }

    fn mk_unary(&self, unop: UnOp, expr: Box<Expr>) -> ExprKind {
        ExprKind::Unary(unop, expr)
    }

    fn mk_binary(&self, binop: BinOp, lhs: Box<Expr>, rhs: Box<Expr>) -> ExprKind {
        ExprKind::Binary(binop, lhs, rhs)
    }

    fn mk_expr(&self, kind: ExprKind, span: Span) -> Box<Expr> {
        Box::new(Expr { kind, span })
    }

    /// Create expression span ensuring the span of the parent node
    /// is larger than the span of lhs and rhs.
    fn mk_expr_sp(&self, lhs: &Box<Expr>, rhs_span: Span) -> Span {
        lhs.span.to(rhs_span)
    }

    fn mk_assign_op(&self, binop: BinOp, lhs: Box<Expr>, rhs: Box<Expr>) -> ExprKind {
        ExprKind::AssignOp(binop, lhs, rhs)
    }
}
