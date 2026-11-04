//! Syntactic validation of contract expressions.
//!
//! The grammar is described in the crate-level documentation of
//! `tpt-invariant-macros`. Errors are reported with `syn::Error`, spanned to the
//! smallest offending sub-expression.

use syn::{BinOp, Expr, Lit, UnOp};

/// Where a contract expression appears. Some constructs are only legal in some
/// contexts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Context {
    Requires,
    Ensures,
    Invariant,
}

impl Context {
    fn name(self) -> &'static str {
        match self {
            Context::Requires => "#[requires]",
            Context::Ensures => "#[ensures]",
            Context::Invariant => "invariant!",
        }
    }
}

/// Checks that `expr` is a well-formed contract expression for `ctx`.
pub(crate) fn check(expr: &Expr, ctx: Context) -> syn::Result<()> {
    check_expr(expr, ctx, false)
}

fn check_expr(expr: &Expr, ctx: Context, in_old: bool) -> syn::Result<()> {
    match expr {
        Expr::Binary(bin) => {
            if !is_allowed_binop(&bin.op) {
                return Err(syn::Error::new_spanned(
                    bin.op,
                    "assignment operators are not allowed in contracts",
                ));
            }
            check_expr(&bin.left, ctx, in_old)?;
            check_expr(&bin.right, ctx, in_old)
        }
        Expr::Unary(un) => {
            if matches!(un.op, UnOp::Deref(_)) {
                return Err(syn::Error::new_spanned(
                    un.op,
                    "dereferencing is not supported in contracts",
                ));
            }
            check_expr(&un.expr, ctx, in_old)
        }
        Expr::Paren(paren) => check_expr(&paren.expr, ctx, in_old),
        Expr::Group(group) => check_expr(&group.expr, ctx, in_old),
        Expr::Lit(lit) => match &lit.lit {
            Lit::Int(_) | Lit::Bool(_) => Ok(()),
            other => Err(syn::Error::new_spanned(
                other,
                "only integer and boolean literals are supported in contracts",
            )),
        },
        Expr::Path(path) => check_path(expr, path, ctx),
        Expr::Call(call) => {
            let calls_old = matches!(&*call.func, Expr::Path(p) if p.path.is_ident("old"));
            if !calls_old {
                return Err(syn::Error::new_spanned(
                    expr,
                    "function calls are not allowed in contracts (only `old(..)` in `ensures`)",
                ));
            }
            check_old(expr, call, ctx, in_old)
        }
        other => Err(syn::Error::new_spanned(
            other,
            format!(
                "unsupported expression in {} (allowed: literals, variables, \
                 arithmetic, comparison, logical and bitwise operators, `result`, `old(..)`)",
                ctx.name()
            ),
        )),
    }
}

fn check_path(expr: &Expr, path: &syn::ExprPath, ctx: Context) -> syn::Result<()> {
    let Some(ident) = path.path.get_ident().filter(|_| path.qself.is_none()) else {
        return Err(syn::Error::new_spanned(
            expr,
            "only plain variable names are supported in contracts",
        ));
    };
    if ident == "result" && ctx != Context::Ensures {
        return Err(syn::Error::new_spanned(
            ident,
            format!(
                "`result` is only available in `ensures`, not in {}",
                ctx.name()
            ),
        ));
    }
    Ok(())
}

fn check_old(expr: &Expr, call: &syn::ExprCall, ctx: Context, in_old: bool) -> syn::Result<()> {
    if ctx != Context::Ensures {
        return Err(syn::Error::new_spanned(
            expr,
            format!(
                "`old(..)` is only available in `ensures`, not in {}",
                ctx.name()
            ),
        ));
    }
    if in_old {
        return Err(syn::Error::new_spanned(expr, "`old(..)` cannot be nested"));
    }
    if call.args.len() != 1 {
        return Err(syn::Error::new_spanned(
            expr,
            "`old` takes exactly one argument",
        ));
    }
    let arg = &call.args[0];
    if !matches!(arg, Expr::Path(_)) {
        return Err(syn::Error::new_spanned(
            arg,
            "`old(..)` takes a plain variable name",
        ));
    }
    check_expr(arg, ctx, true)
}

fn is_allowed_binop(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::Add(_)
            | BinOp::Sub(_)
            | BinOp::Mul(_)
            | BinOp::Div(_)
            | BinOp::Rem(_)
            | BinOp::And(_)
            | BinOp::Or(_)
            | BinOp::BitAnd(_)
            | BinOp::BitOr(_)
            | BinOp::BitXor(_)
            | BinOp::Shl(_)
            | BinOp::Shr(_)
            | BinOp::Eq(_)
            | BinOp::Ne(_)
            | BinOp::Lt(_)
            | BinOp::Le(_)
            | BinOp::Gt(_)
            | BinOp::Ge(_)
    )
}

#[cfg(test)]
mod tests {
    use super::{Context, check};
    use syn::Expr;

    fn ok(src: &str, ctx: Context) {
        let expr: Expr = syn::parse_str(src).expect("test input must parse as an expression");
        if let Err(e) = check(&expr, ctx) {
            panic!("expected `{src}` to be accepted in {ctx:?}, got: {e}");
        }
    }

    fn err(src: &str, ctx: Context) -> String {
        let expr: Expr = syn::parse_str(src).expect("test input must parse as an expression");
        match check(&expr, ctx) {
            Ok(()) => panic!("expected `{src}` to be rejected in {ctx:?}"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn accepts_comparisons_and_logic() {
        ok("x > 0 && y > 0", Context::Requires);
        ok("!(a == b) || c <= 10", Context::Invariant);
        ok("-x < 5", Context::Requires);
    }

    #[test]
    fn accepts_arithmetic_and_bitwise() {
        ok("(a + b * 2) % 7 == 0", Context::Requires);
        ok("(x & 0xff) | (y ^ 1) << 2 >> 1 == 3", Context::Requires);
        ok("true == false", Context::Invariant);
    }

    #[test]
    fn result_only_in_ensures() {
        ok("result >= x && result >= y", Context::Ensures);
        let msg = err("result > 0", Context::Requires);
        assert!(
            msg.contains("`result` is only available in `ensures`"),
            "{msg}"
        );
        let msg = err("result > 0", Context::Invariant);
        assert!(msg.contains("`result`"), "{msg}");
    }

    #[test]
    fn old_only_in_ensures_and_plain_args() {
        ok("result == old(x) + 1", Context::Ensures);
        let msg = err("old(x) > 0", Context::Requires);
        assert!(msg.contains("only available in `ensures`"), "{msg}");
        let msg = err("old(x + 1) > 0", Context::Ensures);
        assert!(msg.contains("plain variable name"), "{msg}");
        let msg = err("old(x, y) > 0", Context::Ensures);
        assert!(msg.contains("exactly one argument"), "{msg}");
        let msg = err("old(old(x)) > 0", Context::Ensures);
        assert!(
            msg.contains("cannot be nested") || msg.contains("plain variable"),
            "{msg}"
        );
    }

    #[test]
    fn rejects_out_of_grammar_expressions() {
        let msg = err("x.len() > 0", Context::Requires);
        assert!(msg.contains("unsupported expression"), "{msg}");
        let msg = err("abs(x) > 0", Context::Requires);
        assert!(msg.contains("function calls are not allowed"), "{msg}");
        let msg = err("a[0] > 0", Context::Requires);
        assert!(msg.contains("unsupported expression"), "{msg}");
        let msg = err("x as u8 > 0", Context::Requires);
        assert!(msg.contains("unsupported expression"), "{msg}");
        let msg = err("x += 1 == 2", Context::Invariant);
        assert!(msg.contains("assignment operators"), "{msg}");
        let msg = err("\"hi\" == s", Context::Requires);
        assert!(msg.contains("integer and boolean literals"), "{msg}");
        let msg = err("s.field == 1", Context::Requires);
        assert!(msg.contains("unsupported expression"), "{msg}");
    }

    #[test]
    fn rejects_paths_that_are_not_plain_names() {
        let msg = err("crate::N > 0", Context::Requires);
        assert!(msg.contains("plain variable names"), "{msg}");
    }
}
