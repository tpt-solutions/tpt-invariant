//! Control-flow graph IR (todo.md, Phase 5).
//!
//! A [`Body`] is a list of basic blocks over the MVP subset: the Rust integer
//! types and `bool`. A block holds assignments and assertions, and ends in one
//! [`Terminator`]. [`crate::lower`] builds bodies from `syn` items.
//!
//! The IR is not in SSA form yet, so a local may be assigned in several blocks.
//! Types are recorded only where the source states them (parameters, `let`
//! annotations, and comparison results). Everything else waits for type
//! inference in a later phase.

use std::fmt::{self, Write as _};

/// A Rust integer type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntTy {
    /// `i8`
    I8,
    /// `i16`
    I16,
    /// `i32`
    I32,
    /// `i64`
    I64,
    /// `i128`
    I128,
    /// `isize`
    Isize,
    /// `u8`
    U8,
    /// `u16`
    U16,
    /// `u32`
    U32,
    /// `u64`
    U64,
    /// `u128`
    U128,
    /// `usize`
    Usize,
}

impl IntTy {
    /// Parses a primitive integer type name such as `"u32"`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "i64" => Self::I64,
            "i128" => Self::I128,
            "isize" => Self::Isize,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            "u128" => Self::U128,
            "usize" => Self::Usize,
            _ => return None,
        })
    }

    /// The primitive name, such as `"u32"`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::I128 => "i128",
            Self::Isize => "isize",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U128 => "u128",
            Self::Usize => "usize",
        }
    }

    /// Whether the type is signed.
    #[must_use]
    pub const fn is_signed(self) -> bool {
        matches!(
            self,
            Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::I128 | Self::Isize
        )
    }

    /// Width in bits. `isize` and `usize` are 64-bit, because the tool assumes
    /// a 64-bit target.
    #[must_use]
    pub const fn bits(self) -> u32 {
        match self {
            Self::I8 | Self::U8 => 8,
            Self::I16 | Self::U16 => 16,
            Self::I32 | Self::U32 => 32,
            Self::I64 | Self::U64 | Self::Isize | Self::Usize => 64,
            Self::I128 | Self::U128 => 128,
        }
    }
}

/// A type in the IR.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    /// An integer type.
    Int(IntTy),
    /// `bool`.
    Bool,
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(int) => f.write_str(int.name()),
            Self::Bool => f.write_str("bool"),
        }
    }
}

/// Index of a local variable or temporary in [`Body::locals`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);

/// Index of a basic block in [`Body::blocks`]. Block 0 is the entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

/// Where a local came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalKind {
    /// A function parameter.
    Param,
    /// A `let` binding or a loop variable.
    User,
    /// A compiler-introduced temporary, named `_tN`.
    Temp,
}

/// Declaration of a local.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalDecl {
    /// Source name, or `_tN` for temporaries.
    pub name: String,
    /// The type, when the source states it. `None` means inference is pending.
    pub ty: Option<Ty>,
    /// Where the local came from.
    pub kind: LocalKind,
}

/// A literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Constant {
    /// An integer literal. `ty` is set only when the literal has a suffix.
    /// Negation is a separate [`UnOp::Neg`], so the value is never negative.
    Int {
        /// The magnitude of the literal.
        value: u128,
        /// The suffix type, if any.
        ty: Option<IntTy>,
    },
    /// `true` or `false`.
    Bool(bool),
}

/// An operand: a constant or a local.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand {
    /// A literal.
    Const(Constant),
    /// The value of a local.
    Local(LocalId),
}

/// Binary operators that do not short-circuit. `&&` and `||` are lowered to
/// control flow instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Rem,
    /// `&`
    BitAnd,
    /// `|`
    BitOr,
    /// `^`
    BitXor,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
}

impl BinOp {
    /// The source symbol.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
        }
    }

    /// Whether the operator yields `bool`.
    #[must_use]
    pub const fn is_comparison(self) -> bool {
        matches!(
            self,
            Self::Eq | Self::Ne | Self::Lt | Self::Le | Self::Gt | Self::Ge
        )
    }
}

/// Unary operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    /// `-`
    Neg,
    /// `!`
    Not,
}

impl UnOp {
    /// The source symbol.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Not => "!",
        }
    }
}

/// The right-hand side of an assignment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rvalue {
    /// Copy of an operand.
    Use(Operand),
    /// A binary operation.
    Binary {
        /// The operator.
        op: BinOp,
        /// Left operand.
        lhs: Operand,
        /// Right operand.
        rhs: Operand,
    },
    /// A unary operation.
    Unary {
        /// The operator.
        op: UnOp,
        /// The operand.
        arg: Operand,
    },
    /// A call of one of the supported builtin methods (`abs`, `min`, `max`,
    /// `wrapping_add`, `wrapping_sub`, `wrapping_mul`).
    Method {
        /// The method name.
        name: String,
        /// The receiver.
        recv: Operand,
        /// The remaining arguments.
        args: Vec<Operand>,
    },
    /// A call of a free function by name. The result is opaque. Contract-based
    /// abstraction is pending (todo.md, Phase 5).
    Call {
        /// The callee name.
        func: String,
        /// The arguments.
        args: Vec<Operand>,
    },
}

/// A non-terminating statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    /// `dst = rvalue`.
    Assign {
        /// The assigned local.
        dst: LocalId,
        /// The value.
        rvalue: Rvalue,
    },
    /// `assert!(cond)` (or `debug_assert!`). Fails when `cond` is false.
    Assert {
        /// The condition.
        cond: Operand,
        /// Source text of the condition, for messages.
        text: String,
    },
}

/// The last statement of a block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Terminator {
    /// Unconditional jump.
    Goto(BlockId),
    /// Jump on a `bool`.
    Branch {
        /// The condition.
        cond: Operand,
        /// Target when the condition is `true`.
        then_bb: BlockId,
        /// Target when the condition is `false`.
        else_bb: BlockId,
    },
    /// Return from the function, with the value when there is one.
    Return(Option<Operand>),
    /// Never reached. Used only for blocks that lowering left without a
    /// terminator, which should not happen.
    Unreachable,
}

/// A basic block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasicBlock {
    /// Statements, in order.
    pub stmts: Vec<Statement>,
    /// The terminator.
    pub term: Terminator,
    /// Source text of `invariant!(..)` annotations on a loop header. Empty
    /// for other blocks. These feed the k-induction step (todo.md, Phase 8).
    pub invariants: Vec<String>,
}

/// The CFG of one function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body {
    /// The function name.
    pub name: String,
    /// Parameter locals, in declaration order.
    pub params: Vec<LocalId>,
    /// The return type, or `None` for unit functions.
    pub ret_ty: Option<Ty>,
    /// All locals, including parameters and temporaries.
    pub locals: Vec<LocalDecl>,
    /// Basic blocks. Block 0 is the entry.
    pub blocks: Vec<BasicBlock>,
}

impl Body {
    /// The declaration of `id`.
    #[must_use]
    pub fn local(&self, id: LocalId) -> &LocalDecl {
        &self.locals[id.0]
    }

    /// Blocks reached directly from `bb`, in terminator order.
    #[must_use]
    pub fn successors(&self, bb: BlockId) -> Vec<BlockId> {
        match &self.blocks[bb.0].term {
            Terminator::Goto(target) => vec![*target],
            Terminator::Branch {
                then_bb, else_bb, ..
            } => vec![*then_bb, *else_bb],
            Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
        }
    }

    /// Renders the CFG in Graphviz DOT, for debugging.
    #[must_use]
    pub fn to_dot(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "digraph \"{}\" {{", dot_escape(&self.name));
        out.push_str("    node [shape=box, fontname=\"monospace\"];\n");
        for (i, block) in self.blocks.iter().enumerate() {
            // `\l` left-justifies each line in DOT. Only the text is escaped.
            let mut label = format!("bb{i}\\l");
            for inv in &block.invariants {
                let _ = write!(label, "invariant {}\\l", dot_escape(inv));
            }
            for stmt in &block.stmts {
                let _ = write!(label, "{}\\l", dot_escape(&self.fmt_statement(stmt)));
            }
            let term = self.fmt_terminator(&block.term);
            let _ = write!(label, "{}\\l", dot_escape(&term));
            let _ = writeln!(out, "    bb{i} [label=\"{label}\"];");
        }
        for (i, block) in self.blocks.iter().enumerate() {
            match &block.term {
                Terminator::Goto(target) => {
                    let _ = writeln!(out, "    bb{i} -> bb{};", target.0);
                }
                Terminator::Branch {
                    then_bb, else_bb, ..
                } => {
                    let _ = writeln!(out, "    bb{i} -> bb{} [label=\"true\"];", then_bb.0);
                    let _ = writeln!(out, "    bb{i} -> bb{} [label=\"false\"];", else_bb.0);
                }
                Terminator::Return(_) | Terminator::Unreachable => {}
            }
        }
        out.push_str("}\n");
        out
    }

    fn fmt_operand(&self, op: &Operand) -> String {
        match op {
            Operand::Local(id) => self.local(*id).name.clone(),
            Operand::Const(Constant::Bool(value)) => value.to_string(),
            Operand::Const(Constant::Int { value, ty }) => match ty {
                Some(ty) => format!("{value}{}", ty.name()),
                None => value.to_string(),
            },
        }
    }

    fn fmt_args(&self, args: &[Operand]) -> String {
        args.iter()
            .map(|arg| self.fmt_operand(arg))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn fmt_rvalue(&self, rvalue: &Rvalue) -> String {
        match rvalue {
            Rvalue::Use(op) => self.fmt_operand(op),
            Rvalue::Binary { op, lhs, rhs } => format!(
                "{} {} {}",
                self.fmt_operand(lhs),
                op.symbol(),
                self.fmt_operand(rhs)
            ),
            Rvalue::Unary { op, arg } => format!("{}{}", op.symbol(), self.fmt_operand(arg)),
            Rvalue::Method { name, recv, args } => {
                format!("{}.{name}({})", self.fmt_operand(recv), self.fmt_args(args))
            }
            Rvalue::Call { func, args } => format!("{func}({})", self.fmt_args(args)),
        }
    }

    fn fmt_statement(&self, stmt: &Statement) -> String {
        match stmt {
            Statement::Assign { dst, rvalue } => {
                format!("{} = {}", self.local(*dst).name, self.fmt_rvalue(rvalue))
            }
            Statement::Assert { text, .. } => format!("assert!({text})"),
        }
    }

    fn fmt_terminator(&self, term: &Terminator) -> String {
        match term {
            Terminator::Goto(target) => format!("goto {target}"),
            Terminator::Branch {
                cond,
                then_bb,
                else_bb,
            } => format!("branch {} ? {then_bb} : {else_bb}", self.fmt_operand(cond)),
            Terminator::Return(Some(op)) => format!("return {}", self.fmt_operand(op)),
            Terminator::Return(None) => "return".to_owned(),
            Terminator::Unreachable => "unreachable".to_owned(),
        }
    }
}

fn dot_escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}
