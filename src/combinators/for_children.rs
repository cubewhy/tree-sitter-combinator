//! The [`ForChildren`] and [`ScanChildren`] combinators — apply a handler to
//! every child of the current node and collect results.
//!
//! These combinators address the "iterate over children and dispatch each one
//! through a kind table" pattern that appears throughout symbol-collection
//! and inlay-hint traversals.
//!
//! ## Two variants
//!
//! | Combinator | Collects | When to use |
//! |---|---|---|
//! | [`ForChildren`] | `Vec<R>` of all `Some` results | Collect every matching child result into one `Vec`. |
//! | [`ScanChildren`] | First `Some` result | Stop at the first child for which the handler succeeds. |
//!
//! Both variants operate on **named children only** by default (i.e. they
//! call `node.named_children()`).  The traversal is shallow — grandchildren
//! are not visited; compose with [`Climb`](super::climb::Climb) or recurse
//! manually if deep traversal is required.

use crate::handler::{Handler, HandlerResult};
use crate::input::Input;

// ---------------------------------------------------------------------------
// ForChildren
// ---------------------------------------------------------------------------

/// A handler that runs `inner` against every **named child** of the current
/// node, collects all `Some(r)` results into a `Vec<R>`, and returns
/// `Some(vec)` (which may be empty).  It never returns `None`.
///
/// This makes it easy to replace `for child in node.named_children(...)` +
/// `match child.kind() { ... }` loops with a composable handler chain.
///
/// Constructed via
/// [`HandlerExt::for_children`](crate::HandlerExt::for_children).
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::{handler_fn, HandlerExt, Input};
///
/// // Collect the kind-string of every named child.
/// let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned())
///     .for_children();
/// let _ = h;
/// ```
pub struct ForChildren<H> {
    /// The handler applied to each named child.
    pub inner: H,
}

impl<Ctx, R, H> Handler<Ctx, Vec<R>> for ForChildren<H>
where
    H: Handler<Ctx, R>,
    Ctx: Copy,
{
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<Vec<R>> {
        let mut results = Vec::new();
        let mut walker = input.node.walk();
        for child in input.node.named_children(&mut walker) {
            if let Some(r) = self.inner.handle(input.with_node(child)) {
                results.push(r);
            }
        }
        Some(results)
    }
}

// ---------------------------------------------------------------------------
// ScanChildren
// ---------------------------------------------------------------------------

/// A handler that runs `inner` against each **named child** of the current
/// node in order, returning the **first** `Some(r)` result.
///
/// If no child produces `Some`, `ScanChildren` returns `None`.
///
/// Constructed via
/// [`HandlerExt::scan_children`](crate::HandlerExt::scan_children).
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::{handler_fn, HandlerExt, Input};
///
/// // Return the kind of the first named child that is an "identifier".
/// let h = (|inp: Input<()>| -> Option<String> {
///     (inp.node.kind() == "identifier").then(|| inp.node.kind().to_owned())
/// })
/// .scan_children();
/// let _ = h;
/// ```
pub struct ScanChildren<H> {
    /// The handler applied to each named child.
    pub inner: H,
}

impl<Ctx, R, H> Handler<Ctx, R> for ScanChildren<H>
where
    H: Handler<Ctx, R>,
    Ctx: Copy,
{
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R> {
        let mut walker = input.node.walk();
        for child in input.node.named_children(&mut walker) {
            if let Some(r) = self.inner.handle(input.with_node(child)) {
                return Some(r);
            }
        }
        None
    }
}
