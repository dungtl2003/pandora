mod ident;
pub mod pretty_print;
mod token;
mod tokenstream;

use core::fmt;
use std::fmt::{Display, Formatter};

pub use ident::Ident;
pub use tokenstream::{DelimSpan, Spacing, TokenStream, TokenTree, TokenTreeCursor};

pub use token::{
    BinOpToken, CommentKind, Delimiter, DocStyle, IdentIsRaw, Lit, LitKind, Token, TokenKind,
};

use crate::span_encoding::{Span, Spanned};

#[derive(Debug)]
pub struct Ast {
    pub stmts: Vec<Box<Stmt>>,
}

impl Ast {
    pub fn new(stmts: Vec<Box<Stmt>>) -> Self {
        Ast { stmts }
    }
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    /// A function declaration: `fn foo() { ... }`.
    FuncDecl(Box<Fun>),
    /// An expression statement: `expr;`.
    Expr(Box<Expr>),
    /// A block statement: `{ stmt* }`.
    Block(Vec<Box<Stmt>>),
    /// An `if` statement: `if expr block_stmt (else (block_stmt | if_stmt))?`.
    If(Box<Expr>, Box<Stmt>, Option<Box<Stmt>>),
    /// A 'break' statement.
    Break,
    /// A 'continue' statement.
    Continue,
    /// A 'return' statement: 'return' expr? ';'
    Return(Option<Box<Expr>>),
    /// A variable declaration: 'var' 'mut'? ident: type ('=' expr)? ';'
    Var(Box<Local>),
    /// A while loop: 'while' expr block_stmt
    While(Box<Expr>, Box<Stmt>),
    /// A for loop: 'for' ident 'in' expr block_stmt
    For(Ident, Box<Expr>, Box<Stmt>),
    /// An import statement: 'import' ident ';'
    Import(Ident),
    /// An empty statement: ';'.
    Empty,
}

#[derive(Debug, Clone)]
pub struct Ty {
    pub kind: TyKind,
    pub span: Span,
}

impl Display for Ty {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.kind {
            TyKind::Array(ty, expr) => match expr {
                Some(expr) => {
                    write!(f, "[{}; {}]", ty, expr)
                }
                None => {
                    write!(f, "[{}]", ty)
                }
            },
            TyKind::Named(ident) => {
                write!(f, "{}", ident.name)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum TyKind {
    /// An array type.
    ///
    /// E.g., `[int; 4]`.
    Array(Box<Ty>, Option<Box<Expr>>),
    /// A named type.
    Named(Ident),
}

/// Local represents a `var` statement. e.g. `var mut <ident>:<ty> = <expr>;`.
#[derive(Debug, Clone)]
pub struct Local {
    pub is_mut: bool,
    pub ident: Ident,
    pub ty: Ty,
    pub kind: LocalKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum LocalKind {
    /// Local declaration.
    /// Example: `let x: int;`
    Decl,
    /// Local declaration with an initializer.
    /// Example: `let x: int = y;`
    Init(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    /// A binary operation (e.g. `a + b`, `a * b`).
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// An unary operation (e.g. `!a`, `-a`).
    Unary(UnOp, Box<Expr>),
    /// A literal (e.g. `124`, `"foo"`).
    Literal(token::Lit),
    /// An assignment (`a = foo()`).
    /// The `Span` argument is the span of the `=` token.
    Assign(Box<Expr>, Box<Expr>, Span),
    /// An assignment with an operator.
    ///
    /// E.g., `a += 1`.
    AssignOp(BinOp, Box<Expr>, Box<Expr>),
    Identifier(Ident),
    /// A cast (e.g., `foo as float`).
    Cast(Box<Expr>, Ty),
    /// A function call.
    ///
    /// The first field resolves to the function itself,
    /// and the second field is the list of arguments.
    FunCall(Box<Expr>, Vec<Box<Expr>>),
    /// Library access (e.g. `foo.bar`).
    LibAccess(Box<Expr>, Ident),
    /// Library function call (e.g. `foo.bar()`).
    LibFunCall(Box<Expr>, Vec<Box<Expr>>),
    /// Array
    Array(Vec<Box<Expr>>),
    /// An indexing operation (e.g., `foo[2]`).
    /// The span represents the span of the `[2]`, including brackets.
    Index(Box<Expr>, Box<Expr>, Span),
    /// An array literal constructed from one repeated element.
    ///
    /// E.g., `[1; 5]`. The left expression is the element to be
    /// repeated; the right expression is the number of times to repeat it.
    Repeat(Box<Expr>, Box<Expr>),
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.kind {
            ExprKind::Binary(op, ..) => {
                write!(f, "Binary operation: {}", op)
            }
            ExprKind::Unary(op, _) => {
                write!(f, "{:?}", op)
            }
            ExprKind::Literal(lit) => {
                write!(f, "{}", lit)
            }
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum UnOp {
    /// The `!` operator for logical inversion.
    Not,
    /// The `-` operator for negation.
    Ne,
}

impl Display for UnOp {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            UnOp::Not => write!(f, "!"),
            UnOp::Ne => write!(f, "-"),
        }
    }
}

pub type BinOp = Spanned<BinOpKind>;

impl BinOp {
    pub fn to_string(&self) -> String {
        (match self.node {
            BinOpKind::Add => "+",
            BinOpKind::Sub => "-",
            BinOpKind::Mul => "*",
            BinOpKind::Div => "/",
            BinOpKind::Mod => "%",
            BinOpKind::Eq => "==",
            BinOpKind::Ne => "!=",
            BinOpKind::Lt => "<",
            BinOpKind::Le => "<=",
            BinOpKind::Gt => ">",
            BinOpKind::Ge => ">=",
            BinOpKind::And => "&&",
            BinOpKind::Or => "||",
            BinOpKind::BitAnd => "&",
            BinOpKind::BitOr => "|",
            BinOpKind::BitXor => "^",
            BinOpKind::Shl => "<<",
            BinOpKind::Shr => ">>",
        })
        .to_string()
    }
}

impl Display for BinOp {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self.node)
    }
}

#[derive(Debug, Clone)]
pub enum BinOpKind {
    /// The `+` operator (addition).
    Add,
    /// The `-` operator (subtraction).
    Sub,
    /// The `*` operator (multiplication).
    Mul,
    /// The `/` operator (division).
    Div,
    /// The `%` operator (modulus).
    Mod,
    /// The `==` operator (equality).
    Eq,
    /// The `!=` operator (not equal to).
    Ne,
    /// The `<` operator (less than).
    Lt,
    /// The `<=` operator (less than or equal to).
    Le,
    /// The `>` operator (greater than).
    Gt,
    /// The `>=` operator (greater than or equal to).
    Ge,
    /// The `&&` operator (and).
    And,
    /// The `||` operator (or).
    Or,
    /// The `&` operator (bitwise and).
    BitAnd,
    /// The `|` operator (bitwise or).
    BitOr,
    /// The `^` operator (bitwise xor).
    BitXor,
    /// The `<<` operator (shift left).
    Shl,
    /// The `>>` operator (shift right).
    Shr,
}

impl Display for BinOpKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl BinOpKind {
    pub fn to_str(&self) -> &str {
        match self {
            BinOpKind::Add => "+",
            BinOpKind::Sub => "-",
            BinOpKind::Mul => "*",
            BinOpKind::Div => "/",
            BinOpKind::Mod => "%",
            BinOpKind::Eq => "==",
            BinOpKind::Ne => "!=",
            BinOpKind::Lt => "<",
            BinOpKind::Le => "<=",
            BinOpKind::Gt => ">",
            BinOpKind::Ge => ">=",
            BinOpKind::And => "&&",
            BinOpKind::Or => "||",
            BinOpKind::BitAnd => "&",
            BinOpKind::BitOr => "|",
            BinOpKind::BitXor => "^",
            BinOpKind::Shl => "<<",
            BinOpKind::Shr => ">>",
        }
    }
}

/// A function definition.
#[derive(Debug, Clone)]
pub struct Fun {
    pub sig: FunSig,
    pub body: Box<Stmt>,
}

/// The signature of a function.
#[derive(Debug, Clone)]
pub struct FunSig {
    pub name: Ident,
    pub inputs: Vec<FunParam>,
    pub output: Option<Ty>,

    pub span: Span,
}

/// A parameter in a function header.
/// E.g., `bar: usize` as in `fn foo(bar: usize)`.
#[derive(Debug, Clone)]
pub struct FunParam {
    pub ty: Ty,
    pub ident: Ident,
    pub is_mut: bool,
    pub span: Span,
}
