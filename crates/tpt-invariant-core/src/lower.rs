//! Lowering from `syn` function items to the [`crate::cfg`] IR (todo.md, Phase 5).
//!
//! The MVP subset: integer and `bool` parameters, `let` with an identifier or
//! `_` pattern and an optional type, assignment and compound assignment to
//! locals, `if` / `else` (including `else if`), short-circuit `&&` and `||`,
//! `while`, `for` over `a..b` and `a..=b`, `return`, calls of free functions,
//! the methods `abs`, `min`, `max`, `wrapping_add`, `wrapping_sub` and
//! `wrapping_mul`, `assert!` and `debug_assert!`, and `invariant!(..)` inside a
//! loop body. Anything else is rejected with a span-accurate
//! "not supported in the MVP subset" error.

use std::fmt::Display;

use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{BinOp as SynBinOp, Expr, FnArg, Lit, Pat, PatIdent, ReturnType, Stmt, Token};

use crate::cfg::{
    BasicBlock, BinOp, BlockId, Body, Constant, IntTy, LocalDecl, LocalId, LocalKind, Operand,
    Rvalue, Statement, Terminator, Ty, UnOp,
};

/// Lowers one free function to a CFG [`Body`].
///
/// # Errors
///
/// Returns a spanned error for generics, `self`, types other than integers and
/// `bool`, and for any construct outside the MVP subset.
pub fn lower_fn(item: &syn::ItemFn) -> syn::Result<Body> {
    let sig = &item.sig;
    if let Some(token) = &sig.constness {
        return unsupported(token, "`const fn`");
    }
    if let Some(token) = &sig.asyncness {
        return unsupported(token, "`async fn`");
    }
    if let Some(token) = &sig.unsafety {
        return unsupported(token, "`unsafe fn`");
    }
    if let Some(abi) = &sig.abi {
        return unsupported(abi, "`extern` functions");
    }
    if !sig.generics.params.is_empty() {
        return unsupported(&sig.generics, "generic functions");
    }
    let ret_ty = match &sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => Some(parse_ty(ty)?),
    };

    let mut builder = Builder::new();
    let mut params = Vec::new();
    for input in &sig.inputs {
        let FnArg::Typed(pat_ty) = input else {
            return unsupported(input, "methods taking `self`");
        };
        let Pat::Ident(ident) = &*pat_ty.pat else {
            return unsupported(&*pat_ty.pat, "non-identifier parameter patterns");
        };
        if !is_plain_ident(ident) {
            return unsupported(ident, "`ref` and `@` parameter patterns");
        }
        let ty = parse_ty(&pat_ty.ty)?;
        let name = ident.ident.to_string();
        let id = builder.new_local(name.clone(), Some(ty), LocalKind::Param);
        builder.bind(name, id);
        params.push(id);
    }

    let value = builder.lower_block_value(&item.block)?;
    // A unit function has no return value, even if its tail expression yields one.
    let value = if ret_ty.is_some() { value } else { None };
    builder.terminate(Terminator::Return(value));
    Ok(builder.finish(sig.ident.to_string(), params, ret_ty))
}

/// Finds the functions in `items` (and in inline `mod` blocks) that carry
/// `#[requires]` or `#[ensures]`. Methods inside `impl` blocks are not
/// returned yet.
#[must_use]
pub fn contracted_fns(items: &[syn::Item]) -> Vec<&syn::ItemFn> {
    let mut out = Vec::new();
    collect_contracted(items, &mut out);
    out
}

fn collect_contracted<'a>(items: &'a [syn::Item], out: &mut Vec<&'a syn::ItemFn>) {
    for item in items {
        match item {
            syn::Item::Fn(func) if has_contract_attr(&func.attrs) => out.push(func),
            syn::Item::Mod(module) => {
                if let Some((_, inner)) = &module.content {
                    collect_contracted(inner, out);
                }
            }
            _ => {}
        }
    }
}

fn has_contract_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "requires" || seg.ident == "ensures")
    })
}

fn err(node: &impl Spanned, message: impl Display) -> syn::Error {
    syn::Error::new(node.span(), message)
}

fn unsupported<T>(node: &impl Spanned, what: impl Display) -> syn::Result<T> {
    Err(err(
        node,
        format!("{what} is not supported in the MVP subset"),
    ))
}

fn is_plain_ident(ident: &PatIdent) -> bool {
    ident.by_ref.is_none() && ident.subpat.is_none()
}

fn local_name(path: &syn::ExprPath) -> Option<String> {
    if path.qself.is_some() {
        return None;
    }
    path.path.get_ident().map(ToString::to_string)
}

fn parse_ty(ty: &syn::Type) -> syn::Result<Ty> {
    if let syn::Type::Path(syn::TypePath {
        qself: None, path, ..
    }) = ty
    {
        if let Some(ident) = path.get_ident() {
            let name = ident.to_string();
            if name == "bool" {
                return Ok(Ty::Bool);
            }
            if let Some(int) = IntTy::from_name(&name) {
                return Ok(Ty::Int(int));
            }
        }
    }
    unsupported(ty, "this type (only integers and `bool` are supported)")
}

fn plain_binop(op: &SynBinOp) -> Option<BinOp> {
    Some(match op {
        SynBinOp::Add(_) => BinOp::Add,
        SynBinOp::Sub(_) => BinOp::Sub,
        SynBinOp::Mul(_) => BinOp::Mul,
        SynBinOp::Div(_) => BinOp::Div,
        SynBinOp::Rem(_) => BinOp::Rem,
        SynBinOp::BitAnd(_) => BinOp::BitAnd,
        SynBinOp::BitOr(_) => BinOp::BitOr,
        SynBinOp::BitXor(_) => BinOp::BitXor,
        SynBinOp::Shl(_) => BinOp::Shl,
        SynBinOp::Shr(_) => BinOp::Shr,
        SynBinOp::Eq(_) => BinOp::Eq,
        SynBinOp::Ne(_) => BinOp::Ne,
        SynBinOp::Lt(_) => BinOp::Lt,
        SynBinOp::Le(_) => BinOp::Le,
        SynBinOp::Gt(_) => BinOp::Gt,
        SynBinOp::Ge(_) => BinOp::Ge,
        _ => return None,
    })
}

fn compound_binop(op: &SynBinOp) -> Option<BinOp> {
    Some(match op {
        SynBinOp::AddAssign(_) => BinOp::Add,
        SynBinOp::SubAssign(_) => BinOp::Sub,
        SynBinOp::MulAssign(_) => BinOp::Mul,
        SynBinOp::DivAssign(_) => BinOp::Div,
        SynBinOp::RemAssign(_) => BinOp::Rem,
        SynBinOp::BitAndAssign(_) => BinOp::BitAnd,
        SynBinOp::BitOrAssign(_) => BinOp::BitOr,
        SynBinOp::BitXorAssign(_) => BinOp::BitXor,
        SynBinOp::ShlAssign(_) => BinOp::Shl,
        SynBinOp::ShrAssign(_) => BinOp::Shr,
        _ => return None,
    })
}

/// Mutable state while one function is lowered.
struct Builder {
    locals: Vec<LocalDecl>,
    blocks: Vec<PendingBlock>,
    cur: BlockId,
    scopes: Vec<Vec<(String, LocalId)>>,
    /// Header blocks of the enclosing loops, innermost last.
    loop_headers: Vec<BlockId>,
}

#[derive(Default)]
struct PendingBlock {
    stmts: Vec<Statement>,
    term: Option<Terminator>,
    invariants: Vec<String>,
}

impl Builder {
    fn new() -> Self {
        let mut builder = Self {
            locals: Vec::new(),
            blocks: Vec::new(),
            cur: BlockId(0),
            scopes: vec![Vec::new()],
            loop_headers: Vec::new(),
        };
        builder.new_block(); // the entry block, BlockId(0)
        builder
    }

    fn finish(self, name: String, params: Vec<LocalId>, ret_ty: Option<Ty>) -> Body {
        let blocks = self
            .blocks
            .into_iter()
            .map(|block| BasicBlock {
                stmts: block.stmts,
                term: block.term.unwrap_or(Terminator::Unreachable),
                invariants: block.invariants,
            })
            .collect();
        Body {
            name,
            params,
            ret_ty,
            locals: self.locals,
            blocks,
        }
    }

    fn new_block(&mut self) -> BlockId {
        self.blocks.push(PendingBlock::default());
        BlockId(self.blocks.len() - 1)
    }

    fn new_local(&mut self, name: String, ty: Option<Ty>, kind: LocalKind) -> LocalId {
        self.locals.push(LocalDecl { name, ty, kind });
        LocalId(self.locals.len() - 1)
    }

    fn temp(&mut self, ty: Option<Ty>) -> LocalId {
        let name = format!("_t{}", self.locals.len());
        self.new_local(name, ty, LocalKind::Temp)
    }

    fn emit(&mut self, stmt: Statement) {
        self.blocks[self.cur.0].stmts.push(stmt);
    }

    fn assign_temp(&mut self, rvalue: Rvalue, ty: Option<Ty>) -> LocalId {
        let dst = self.temp(ty);
        self.emit(Statement::Assign { dst, rvalue });
        dst
    }

    fn terminate(&mut self, term: Terminator) {
        let block = &mut self.blocks[self.cur.0];
        debug_assert!(block.term.is_none(), "block terminated twice");
        block.term = Some(term);
    }

    fn goto(&mut self, target: BlockId) {
        self.terminate(Terminator::Goto(target));
    }

    fn switch_to(&mut self, bb: BlockId) {
        self.cur = bb;
    }

    /// After a `return`, code that follows is unreachable. It goes into a
    /// fresh block with no predecessors.
    fn switch_to_dead_block(&mut self) {
        let dead = self.new_block();
        self.switch_to(dead);
    }

    fn push_scope(&mut self) {
        self.scopes.push(Vec::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn bind(&mut self, name: String, id: LocalId) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.push((name, id));
        }
    }

    fn lookup(&self, name: &str) -> Option<LocalId> {
        self.scopes
            .iter()
            .rev()
            .flat_map(|scope| scope.iter().rev())
            .find(|entry| entry.0 == name)
            .map(|entry| entry.1)
    }

    /// Lowers a block. Its tail expression, if any, is the block's value.
    fn lower_block_value(&mut self, block: &syn::Block) -> syn::Result<Option<Operand>> {
        self.push_scope();
        let mut value = None;
        let last = block.stmts.len().saturating_sub(1);
        for (i, stmt) in block.stmts.iter().enumerate() {
            match stmt {
                Stmt::Local(local) => self.lower_let(local)?,
                Stmt::Expr(expr, None) if i == last => value = self.lower_expr(expr)?,
                Stmt::Expr(expr, _) => {
                    self.lower_expr(expr)?;
                }
                Stmt::Macro(stmt_mac) => self.lower_macro(&stmt_mac.mac)?,
                Stmt::Item(item) => return unsupported(item, "items inside function bodies"),
            }
        }
        self.pop_scope();
        Ok(value)
    }

    fn lower_let(&mut self, local: &syn::Local) -> syn::Result<()> {
        let (pat, ty) = match &local.pat {
            Pat::Type(pat_ty) => (&*pat_ty.pat, Some(parse_ty(&pat_ty.ty)?)),
            other => (other, None),
        };
        let Some(init) = &local.init else {
            return unsupported(local, "`let` without an initializer");
        };
        if init.diverge.is_some() {
            return unsupported(local, "`let ... else`");
        }
        let value = self.lower_value(&init.expr)?;
        match pat {
            Pat::Wild(_) => {}
            Pat::Ident(ident) if is_plain_ident(ident) => {
                let name = ident.ident.to_string();
                let id = self.new_local(name.clone(), ty, LocalKind::User);
                self.emit(Statement::Assign {
                    dst: id,
                    rvalue: Rvalue::Use(value),
                });
                self.bind(name, id);
            }
            other => return unsupported(other, "this `let` pattern"),
        }
        Ok(())
    }

    /// Lowers an expression that must produce a value.
    fn lower_value(&mut self, expr: &Expr) -> syn::Result<Operand> {
        self.lower_expr(expr)?
            .ok_or_else(|| err(expr, "this expression has no value"))
    }

    /// Lowers an expression. Returns `None` for unit-valued expressions.
    fn lower_expr(&mut self, expr: &Expr) -> syn::Result<Option<Operand>> {
        match expr {
            Expr::Lit(lit) => match &lit.lit {
                Lit::Int(int) => {
                    let value = int.base10_parse::<u128>().map_err(|_| {
                        err(
                            int,
                            "only decimal integer literals that fit in u128 are supported",
                        )
                    })?;
                    let ty = match int.suffix() {
                        "" => None,
                        suffix => Some(
                            IntTy::from_name(suffix)
                                .ok_or_else(|| err(int, "unsupported literal suffix"))?,
                        ),
                    };
                    Ok(Some(Operand::Const(Constant::Int { value, ty })))
                }
                Lit::Bool(boolean) => Ok(Some(Operand::Const(Constant::Bool(boolean.value)))),
                other => unsupported(other, "literals other than integers and `bool`"),
            },
            Expr::Paren(paren) => self.lower_expr(&paren.expr),
            Expr::Group(group) => self.lower_expr(&group.expr),
            Expr::Path(path) => match local_name(path) {
                Some(name) => self
                    .lookup(&name)
                    .map(|id| Some(Operand::Local(id)))
                    .ok_or_else(|| {
                        err(
                            path,
                            format!(
                                "unknown name `{name}`: constants and paths are not supported in the MVP subset"
                            ),
                        )
                    }),
                None => unsupported(path, "paths other than local variables"),
            },
            Expr::Unary(unary) => {
                let op = match unary.op {
                    syn::UnOp::Neg(_) => UnOp::Neg,
                    syn::UnOp::Not(_) => UnOp::Not,
                    syn::UnOp::Deref(_) => return unsupported(&unary.op, "dereferencing"),
                    _ => return unsupported(&unary.op, "this unary operator"),
                };
                let arg = self.lower_value(&unary.expr)?;
                Ok(Some(Operand::Local(
                    self.assign_temp(Rvalue::Unary { op, arg }, None),
                )))
            }
            Expr::Binary(bin) => self.lower_binary(bin),
            Expr::Assign(assign) => {
                let dst = self.lower_place(&assign.left)?;
                let value = self.lower_value(&assign.right)?;
                self.emit(Statement::Assign {
                    dst,
                    rvalue: Rvalue::Use(value),
                });
                Ok(None)
            }
            Expr::If(if_expr) => self.lower_if(if_expr),
            Expr::Block(block) => {
                if block.label.is_some() {
                    return unsupported(block, "labeled blocks");
                }
                self.lower_block_value(&block.block)
            }
            Expr::While(while_expr) => {
                if while_expr.label.is_some() {
                    return unsupported(while_expr, "labeled loops");
                }
                self.lower_while(while_expr)
            }
            Expr::ForLoop(for_expr) => self.lower_for(for_expr),
            Expr::Return(ret) => {
                let value = match &ret.expr {
                    Some(expr) => self.lower_expr(expr)?,
                    None => None,
                };
                self.terminate(Terminator::Return(value));
                self.switch_to_dead_block();
                Ok(None)
            }
            Expr::Call(call) => self.lower_call(call),
            Expr::MethodCall(method) => self.lower_method(method),
            Expr::Macro(mac) => {
                self.lower_macro(&mac.mac)?;
                Ok(None)
            }
            Expr::Loop(_) => unsupported(expr, "`loop`"),
            Expr::Match(_) => unsupported(expr, "`match`"),
            Expr::Break(_) => unsupported(expr, "`break`"),
            Expr::Continue(_) => unsupported(expr, "`continue`"),
            Expr::Let(_) => unsupported(expr, "`if let` and `while let`"),
            Expr::Cast(_) => unsupported(expr, "`as` casts"),
            Expr::Index(_) => unsupported(expr, "indexing"),
            Expr::Field(_) => unsupported(expr, "field access"),
            Expr::Closure(_) => unsupported(expr, "closures"),
            Expr::Range(_) => unsupported(expr, "ranges outside `for` loops"),
            _ => unsupported(expr, "this expression form"),
        }
    }

    fn lower_place(&self, expr: &Expr) -> syn::Result<LocalId> {
        if let Expr::Path(path) = expr {
            if let Some(id) = local_name(path).and_then(|name| self.lookup(&name)) {
                return Ok(id);
            }
        }
        unsupported(expr, "assignment to this place (only local variables)")
    }

    fn lower_binary(&mut self, bin: &syn::ExprBinary) -> syn::Result<Option<Operand>> {
        match &bin.op {
            SynBinOp::And(_) => self.lower_short_circuit(bin, true),
            SynBinOp::Or(_) => self.lower_short_circuit(bin, false),
            op => {
                if let Some(bin_op) = plain_binop(op) {
                    let lhs = self.lower_value(&bin.left)?;
                    let rhs = self.lower_value(&bin.right)?;
                    let ty = bin_op.is_comparison().then_some(Ty::Bool);
                    let rvalue = Rvalue::Binary {
                        op: bin_op,
                        lhs,
                        rhs,
                    };
                    Ok(Some(Operand::Local(self.assign_temp(rvalue, ty))))
                } else if let Some(bin_op) = compound_binop(op) {
                    let dst = self.lower_place(&bin.left)?;
                    let rhs = self.lower_value(&bin.right)?;
                    let rvalue = Rvalue::Binary {
                        op: bin_op,
                        lhs: Operand::Local(dst),
                        rhs,
                    };
                    self.emit(Statement::Assign { dst, rvalue });
                    Ok(None)
                } else {
                    unsupported(op, "this operator")
                }
            }
        }
    }

    /// `a && b` (when `is_and`) or `a || b`, lowered to control flow so that
    /// `b` runs only when needed.
    fn lower_short_circuit(
        &mut self,
        bin: &syn::ExprBinary,
        is_and: bool,
    ) -> syn::Result<Option<Operand>> {
        let lhs = self.lower_value(&bin.left)?;
        let result = self.temp(Some(Ty::Bool));
        self.emit(Statement::Assign {
            dst: result,
            rvalue: Rvalue::Use(lhs),
        });
        let rhs_bb = self.new_block();
        let join = self.new_block();
        let (then_bb, else_bb) = if is_and {
            (rhs_bb, join)
        } else {
            (join, rhs_bb)
        };
        self.terminate(Terminator::Branch {
            cond: Operand::Local(result),
            then_bb,
            else_bb,
        });
        self.switch_to(rhs_bb);
        let rhs = self.lower_value(&bin.right)?;
        self.emit(Statement::Assign {
            dst: result,
            rvalue: Rvalue::Use(rhs),
        });
        self.goto(join);
        self.switch_to(join);
        Ok(Some(Operand::Local(result)))
    }

    fn lower_if(&mut self, if_expr: &syn::ExprIf) -> syn::Result<Option<Operand>> {
        let cond = self.lower_value(&if_expr.cond)?;
        let then_bb = self.new_block();
        let else_bb = self.new_block();
        let join = self.new_block();
        self.terminate(Terminator::Branch {
            cond,
            then_bb,
            else_bb,
        });
        let result = self.temp(None);
        let mut has_value = false;

        self.switch_to(then_bb);
        if let Some(value) = self.lower_block_value(&if_expr.then_branch)? {
            self.emit(Statement::Assign {
                dst: result,
                rvalue: Rvalue::Use(value),
            });
            has_value = true;
        }
        self.goto(join);

        self.switch_to(else_bb);
        if let Some((_, else_expr)) = &if_expr.else_branch {
            if let Some(value) = self.lower_expr(else_expr)? {
                self.emit(Statement::Assign {
                    dst: result,
                    rvalue: Rvalue::Use(value),
                });
                has_value = true;
            }
        }
        self.goto(join);

        self.switch_to(join);
        // A branch that returns has no value. The other branch's type applies.
        Ok((if_expr.else_branch.is_some() && has_value).then_some(Operand::Local(result)))
    }

    fn lower_while(&mut self, while_expr: &syn::ExprWhile) -> syn::Result<Option<Operand>> {
        let header = self.new_block();
        self.goto(header);
        self.switch_to(header);
        let cond = self.lower_value(&while_expr.cond)?;
        let body = self.new_block();
        let exit = self.new_block();
        self.terminate(Terminator::Branch {
            cond,
            then_bb: body,
            else_bb: exit,
        });

        self.switch_to(body);
        self.loop_headers.push(header);
        self.lower_block_value(&while_expr.body)?;
        self.loop_headers.pop();
        self.goto(header);

        self.switch_to(exit);
        Ok(None)
    }

    /// `for x in a..b` and `for x in a..=b`. The end bound is evaluated once,
    /// and a hidden counter drives the loop. The inclusive form checks for the
    /// last value before incrementing, so `..=MAX` does not overflow.
    fn lower_for(&mut self, for_expr: &syn::ExprForLoop) -> syn::Result<Option<Operand>> {
        let Expr::Range(range) = &*for_expr.expr else {
            return unsupported(
                &*for_expr.expr,
                "`for` over anything other than an integer range",
            );
        };
        let (Some(start), Some(end)) = (&range.start, &range.end) else {
            return unsupported(range, "open-ended ranges");
        };
        let binding = match &*for_expr.pat {
            Pat::Ident(ident) if is_plain_ident(ident) => Some(ident.ident.to_string()),
            Pat::Wild(_) => None,
            other => return unsupported(other, "this `for` pattern"),
        };
        let closed = matches!(range.limits, syn::RangeLimits::Closed(_));

        let start_value = self.lower_value(start)?;
        let end_value = self.lower_value(end)?;
        let end_tmp = self.temp(None);
        self.emit(Statement::Assign {
            dst: end_tmp,
            rvalue: Rvalue::Use(end_value),
        });
        let counter = self.temp(None);
        self.emit(Statement::Assign {
            dst: counter,
            rvalue: Rvalue::Use(start_value),
        });

        let header = self.new_block();
        self.goto(header);
        self.switch_to(header);
        let cmp = if closed { BinOp::Le } else { BinOp::Lt };
        let cond = self.assign_temp(
            Rvalue::Binary {
                op: cmp,
                lhs: Operand::Local(counter),
                rhs: Operand::Local(end_tmp),
            },
            Some(Ty::Bool),
        );
        let body = self.new_block();
        let latch = self.new_block();
        let exit = self.new_block();
        self.terminate(Terminator::Branch {
            cond: Operand::Local(cond),
            then_bb: body,
            else_bb: exit,
        });

        self.switch_to(body);
        self.push_scope();
        if let Some(name) = binding {
            let var = self.new_local(name.clone(), None, LocalKind::User);
            self.emit(Statement::Assign {
                dst: var,
                rvalue: Rvalue::Use(Operand::Local(counter)),
            });
            self.bind(name, var);
        }
        self.loop_headers.push(header);
        self.lower_block_value(&for_expr.body)?;
        self.loop_headers.pop();
        self.pop_scope();
        self.goto(latch);

        self.switch_to(latch);
        if closed {
            let at_last = self.assign_temp(
                Rvalue::Binary {
                    op: BinOp::Eq,
                    lhs: Operand::Local(counter),
                    rhs: Operand::Local(end_tmp),
                },
                Some(Ty::Bool),
            );
            let incr = self.new_block();
            self.terminate(Terminator::Branch {
                cond: Operand::Local(at_last),
                then_bb: exit,
                else_bb: incr,
            });
            self.switch_to(incr);
        }
        self.emit(Statement::Assign {
            dst: counter,
            rvalue: Rvalue::Binary {
                op: BinOp::Add,
                lhs: Operand::Local(counter),
                rhs: Operand::Const(Constant::Int { value: 1, ty: None }),
            },
        });
        self.goto(header);

        self.switch_to(exit);
        Ok(None)
    }

    fn lower_call(&mut self, call: &syn::ExprCall) -> syn::Result<Option<Operand>> {
        let Expr::Path(path) = &*call.func else {
            return unsupported(call, "calls through non-path callees");
        };
        let Some(func) = local_name(path) else {
            return unsupported(call, "calls through qualified or generic paths");
        };
        if self.lookup(&func).is_some() {
            return unsupported(call, "calling a local variable");
        }
        let args = call
            .args
            .iter()
            .map(|arg| self.lower_value(arg))
            .collect::<syn::Result<Vec<_>>>()?;
        Ok(Some(Operand::Local(
            self.assign_temp(Rvalue::Call { func, args }, None),
        )))
    }

    fn lower_method(&mut self, method: &syn::ExprMethodCall) -> syn::Result<Option<Operand>> {
        let name = method.method.to_string();
        let arity = match name.as_str() {
            "abs" => 0,
            "min" | "max" | "wrapping_add" | "wrapping_sub" | "wrapping_mul" => 1,
            _ => return unsupported(&method.method, format!("the method `{name}`")),
        };
        if method.turbofish.is_some() || method.args.len() != arity {
            return unsupported(method, "this form of method call");
        }
        let recv = self.lower_value(&method.receiver)?;
        let args = method
            .args
            .iter()
            .map(|arg| self.lower_value(arg))
            .collect::<syn::Result<Vec<_>>>()?;
        Ok(Some(Operand::Local(
            self.assign_temp(Rvalue::Method { name, recv, args }, None),
        )))
    }

    /// `assert!` and `debug_assert!` (debug semantics), and `invariant!` in a
    /// loop body. Other macros are rejected.
    fn lower_macro(&mut self, mac: &syn::Macro) -> syn::Result<()> {
        let name = mac
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_default();
        match name.as_str() {
            "assert" | "debug_assert" => {
                let args = mac.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)?;
                let mut args = args.into_iter();
                let Some(cond_expr) = args.next() else {
                    return Err(err(mac, "`assert!` needs a condition"));
                };
                let rest: Vec<Expr> = args.collect();
                let message_is_literal = match rest.as_slice() {
                    [] => true,
                    [Expr::Lit(lit)] => matches!(lit.lit, Lit::Str(_)),
                    _ => false,
                };
                if !message_is_literal {
                    return unsupported(mac, "`assert!` messages with format arguments");
                }
                let cond = self.lower_value(&cond_expr)?;
                self.emit(Statement::Assert {
                    cond,
                    text: cond_expr.to_token_stream().to_string(),
                });
                Ok(())
            }
            "invariant" => {
                let inv: Expr = mac.parse_body()?;
                let Some(&header) = self.loop_headers.last() else {
                    return Err(err(mac, "`invariant!` is only allowed inside a loop body"));
                };
                self.blocks[header.0]
                    .invariants
                    .push(inv.to_token_stream().to_string());
                Ok(())
            }
            other => unsupported(mac, format!("the `{other}!` macro")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lower(src: &str) -> syn::Result<Body> {
        let item: syn::ItemFn = syn::parse_str(src).expect("test source parses");
        lower_fn(&item)
    }

    fn lower_ok(src: &str) -> Body {
        lower(src).unwrap_or_else(|err| panic!("unexpected error: {err}"))
    }

    fn rejection(src: &str) -> String {
        match lower(src) {
            Ok(_) => panic!("expected `{src}` to be rejected"),
            Err(err) => err.to_string(),
        }
    }

    fn has_binop(body: &Body, wanted: BinOp) -> bool {
        body.blocks.iter().flat_map(|b| &b.stmts).any(|stmt| {
            matches!(
                stmt,
                Statement::Assign {
                    rvalue: Rvalue::Binary { op, .. },
                    ..
                } if *op == wanted
            )
        })
    }

    #[test]
    fn max_lowers_to_a_diamond() {
        let body = lower_ok("fn max(x: i32, y: i32) -> i32 { if x > y { x } else { y } }");
        assert_eq!(body.blocks.len(), 4);
        assert_eq!(body.params.len(), 2);
        assert_eq!(body.ret_ty, Some(Ty::Int(IntTy::I32)));
        assert!(matches!(body.blocks[0].term, Terminator::Branch { .. }));
        assert_eq!(body.successors(BlockId(1)), vec![BlockId(3)]);
        assert_eq!(body.successors(BlockId(2)), vec![BlockId(3)]);
        assert!(matches!(
            body.blocks[3].term,
            Terminator::Return(Some(Operand::Local(_)))
        ));
    }

    #[test]
    fn abs_uses_unary_negation() {
        let body = lower_ok("fn abs(x: i32) -> i32 { if x < 0 { -x } else { x } }");
        let negated = body.blocks.iter().flat_map(|b| &b.stmts).any(|stmt| {
            matches!(
                stmt,
                Statement::Assign {
                    rvalue: Rvalue::Unary { op: UnOp::Neg, .. },
                    ..
                }
            )
        });
        assert!(negated);
    }

    #[test]
    fn logical_and_is_short_circuit() {
        let body = lower_ok("fn both(a: bool, b: bool) -> bool { a && b }");
        assert_eq!(body.blocks.len(), 3);
        assert_eq!(
            body.blocks[0].term,
            Terminator::Branch {
                cond: Operand::Local(LocalId(2)),
                then_bb: BlockId(1),
                else_bb: BlockId(2),
            }
        );
        assert_eq!(body.successors(BlockId(1)), vec![BlockId(2)]);
    }

    #[test]
    fn while_loop_has_back_edge() {
        let body = lower_ok(
            "fn sum_to(n: u32) -> u32 { let mut s = 0; let mut i = 0; while i < n { s += i; i += 1; } s }",
        );
        // entry, header, body, exit
        assert_eq!(body.blocks.len(), 4);
        assert!(matches!(
            body.blocks[1].term,
            Terminator::Branch {
                then_bb: BlockId(2),
                else_bb: BlockId(3),
                ..
            }
        ));
        assert_eq!(body.blocks[2].term, Terminator::Goto(BlockId(1)));
        assert!(has_binop(&body, BinOp::Add));
    }

    #[test]
    fn exclusive_range_loop_uses_less_than() {
        let body =
            lower_ok("fn total(n: u32) -> u32 { let mut c = 0; for i in 0..n { c += i; } c }");
        assert!(has_binop(&body, BinOp::Lt));
        assert!(!has_binop(&body, BinOp::Eq));
        assert!(body.locals.iter().any(|local| local.name == "i"));
    }

    #[test]
    fn inclusive_range_loop_checks_the_last_value_before_incrementing() {
        let body =
            lower_ok("fn count(n: u8) -> u32 { let mut c = 0; for _ in 0..=n { c += 1; } c }");
        assert!(has_binop(&body, BinOp::Le));
        assert!(has_binop(&body, BinOp::Eq));
    }

    #[test]
    fn assert_records_source_text() {
        let body = lower_ok("fn pos(x: i32) -> i32 { assert!(x != 0); x }");
        // The condition's temporary is assigned before the assert itself.
        assert!(body.blocks[0].stmts.iter().any(|stmt| matches!(
            stmt,
            Statement::Assert { text, .. } if text.contains('x')
        )));
    }

    #[test]
    fn invariant_attaches_to_loop_header() {
        let body = lower_ok(
            "fn count_up(n: i32) -> i32 { let mut i = 0; while i < n { invariant!(i >= 0); i += 1; } i }",
        );
        assert_eq!(body.blocks[1].invariants, vec!["i >= 0".to_owned()]);
    }

    #[test]
    fn dot_dump_lists_labelled_edges() {
        let body = lower_ok("fn max(x: i32, y: i32) -> i32 { if x > y { x } else { y } }");
        let dot = body.to_dot();
        assert!(dot.starts_with("digraph \"max\" {"));
        assert!(dot.contains("bb0 -> bb1 [label=\"true\"];"));
        assert!(dot.contains("bb0 -> bb2 [label=\"false\"];"));
        assert!(dot.contains("bb1 -> bb3;"));
    }

    #[test]
    fn example_corpus_lowers() {
        for src in [
            "fn min(x: i32, y: i32) -> i32 { if x < y { x } else { y } }",
            "fn clamp(x: i32, lo: i32, hi: i32) -> i32 { if x < lo { lo } else if x > hi { hi } else { x } }",
            "fn gcd(mut a: u32, mut b: u32) -> u32 { while b != 0 { let t = b; b = a % b; a = t; } a }",
            "fn wrap_add(a: u8, b: u8) -> u8 { a.wrapping_add(b) }",
            "fn magnitude(x: i64) -> i64 { x.abs().max(0) }",
        ] {
            lower_ok(src);
        }
    }

    #[test]
    fn rejects_loop_and_match() {
        assert!(rejection("fn f() -> i32 { loop { return 1; } }").contains("`loop`"));
        assert!(rejection("fn f(x: i32) -> i32 { match x { _ => 1 } }").contains("`match`"));
    }

    #[test]
    fn rejects_break_and_if_let() {
        assert!(rejection("fn f() { while true { break; } }").contains("`break`"));
        assert!(
            rejection("fn f(x: i32) -> i32 { if let 1 = x { 1 } else { 2 } }").contains("`if let`")
        );
    }

    #[test]
    fn rejects_unsupported_types_and_receivers() {
        assert!(rejection("fn f(s: String) -> i32 { 0 }").contains("this type"));
        assert!(rejection("fn f(self) -> i32 { 0 }").contains("`self`"));
        assert!(rejection("fn f<T>(x: i32) -> i32 { x }").contains("generic functions"));
    }

    #[test]
    fn rejects_casts_and_unknown_methods() {
        assert!(rejection("fn f(x: u8) -> u32 { x as u32 }").contains("`as` casts"));
        assert!(rejection("fn f(x: i32) -> i32 { x.pow(2) }").contains("`pow`"));
    }

    #[test]
    fn contracted_fns_finds_annotated_functions_in_modules() {
        let file: syn::File = syn::parse_str(
            "#[requires(x > 0)] fn a(x: i32) -> i32 { x }
             fn b() {}
             mod m { #[ensures(result > 0)] pub fn c() -> i32 { 1 } }",
        )
        .expect("test source parses");
        let names: Vec<String> = contracted_fns(&file.items)
            .iter()
            .map(|func| func.sig.ident.to_string())
            .collect();
        assert_eq!(names, ["a", "c"]);
    }
}
