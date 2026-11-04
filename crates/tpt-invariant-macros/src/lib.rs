//! Contract attribute macros for `tpt-invariant`.
//!
//! This crate is a proc-macro crate, so it can only export macros. Users
//! normally reach these through the `tpt_invariant::contracts` facade.
//!
//! # Contract grammar
//!
//! | Form | Where | Meaning |
//! |------|-------|---------|
//! | `#[requires(e)]` | functions | precondition, may read parameters only |
//! | `#[ensures(e)]` | functions | postcondition, may use `result` and `old(p)` |
//! | `invariant!(e)` | loop bodies | loop invariant, expands to `()` |
//!
//! Expressions may contain:
//! - integer and boolean literals, and plain variable names;
//! - unary `!` and `-`, and parentheses;
//! - binary arithmetic (`+ - * / %`), bitwise (`& | ^ << >>`), comparison
//!   (`== != < <= > >=`) and logical (`&& ||`) operators;
//! - `result` (only in `ensures`);
//! - `old(p)` (only in `ensures`, with a single plain-variable argument, not
//!   nested).
//!
//! Anything else (method calls, indexing, field access, other function calls,
//! assignments, `as` casts, ...) is rejected at compile time with a diagnostic
//! that points at the offending sub-expression.
//!
//! # Zero cost
//!
//! The contract attributes validate their argument and then re-emit the
//! annotated function unchanged. They do not add code, so normal builds are
//! unaffected.
//!
//! # Scope of this crate
//!
//! This crate only checks the *syntactic* grammar above. Name resolution,
//! typing and lowering to the verifier IR belong to `tpt-invariant-core`.
//! `proc-macro` crates cannot export ordinary items, so the typed contract AST
//! is defined in the core crate rather than here.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, ItemFn};

mod grammar;

use grammar::Context;

/// Declares a precondition on a function.
///
/// ```ignore
/// #[requires(x > 0 && y > 0)]
/// fn f(x: i32, y: i32) -> i32 { x + y }
/// ```
#[proc_macro_attribute]
pub fn requires(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_fn_contract(attr.into(), item.into(), Context::Requires).into()
}

/// Declares a postcondition on a function. `result` names the return value and
/// `old(p)` refers to a parameter's value on entry.
///
/// ```ignore
/// #[ensures(result >= x && result >= y)]
/// fn max(x: i32, y: i32) -> i32 { if x > y { x } else { y } }
/// ```
#[proc_macro_attribute]
pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_fn_contract(attr.into(), item.into(), Context::Ensures).into()
}

/// Declares a loop invariant. Use it as a statement inside a `while`, `for` or
/// `loop` body. It expands to `()`, so it has no runtime effect.
///
/// ```ignore
/// while i < n {
///     invariant!(i <= n);
///     i += 1;
/// }
/// ```
#[proc_macro]
pub fn invariant(input: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    match syn::parse2::<Expr>(input).and_then(|expr| grammar::check(&expr, Context::Invariant)) {
        Ok(()) => quote!(()).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Validates a function contract attribute and re-emits the function.
///
/// On error the function is still emitted, next to the diagnostic. This avoids
/// a cascade of "cannot find function" errors at every call site.
fn expand_fn_contract(
    attr: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
    ctx: Context,
) -> proc_macro2::TokenStream {
    let checked = syn::parse2::<Expr>(attr)
        .and_then(|expr| grammar::check(&expr, ctx))
        .and_then(|()| {
            syn::parse2::<ItemFn>(item.clone())
                .map(|_| ())
                .map_err(|e| {
                    syn::Error::new(
                        e.span(),
                        "contract attributes can only be applied to a function",
                    )
                })
        });

    match checked {
        Ok(()) => item,
        Err(err) => {
            let diagnostic = err.to_compile_error();
            quote! {
                #diagnostic
                #item
            }
        }
    }
}
