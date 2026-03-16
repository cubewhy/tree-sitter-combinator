//! The [`FindAncestor`] combinator and [`HandlerExt::find_ancestor`] method.
//!
//! This combinator solves the "find the nearest ancestor of a specific kind,
//! then run a handler on *that* node" pattern, which is distinct from
//! [`Climb`](super::climb::Climb).
//!
//! `Climb` retries **the same handler** on successive ancestors until it
//! returns `Some`.  `FindAncestor` first **locates** the nearest ancestor
//! whose kind is in `target_kinds`, then runs `inner` exactly once on that
//! found node.  If no such ancestor exists, it returns `None`.
//!
//! ## Comparison table
//!
//! | Combinator | When to use |
//! |---|---|
//! | `.climb(stop)` | Retry `self` on each ancestor until it succeeds. |
//! | `.find_ancestor(kinds, stop)` | Find the nearest ancestor of a specific kind, then run `self` on it once. |

use crate::handler::{Handler, HandlerResult};
use crate::input::Input;

/// A handler that walks up the syntax tree to find the nearest strict
/// ancestor whose kind is in `target_kinds`, then runs `inner` on that
/// ancestor node.
///
/// **The original node is never tested** — only its ancestors are examined.
///
/// Walking stops (returning `None`) when:
/// - a node whose kind is in `stop_kinds` is reached, or
/// - the root is reached with no match.
///
/// Constructed via
/// [`HandlerExt::find_ancestor`](crate::HandlerExt::find_ancestor).
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::{handler_fn, HandlerExt, Input};
///
/// // Walk up to the nearest "argument_list" and return its kind.
/// let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned())
///     .find_ancestor(&["argument_list"], &["program"]);
/// let _ = h;
/// ```
pub struct FindAncestor<H> {
    /// The handler invoked on the found ancestor node.
    pub inner: H,
    /// The set of ancestor kinds to search for.  The first ancestor whose
    /// kind appears in this list wins.
    pub target_kinds: &'static [&'static str],
    /// Ancestor kinds at which the search stops unconditionally (the stop
    /// node itself is **not** tested against `target_kinds`).
    pub stop_kinds: &'static [&'static str],
}

impl<Ctx, R, H> Handler<Ctx, R> for FindAncestor<H>
where
    H: Handler<Ctx, R>,
    Ctx: Copy,
{
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R> {
        let mut current = input.node.parent();
        while let Some(ancestor) = current {
            if self.stop_kinds.contains(&ancestor.kind()) {
                return None;
            }
            if self.target_kinds.contains(&ancestor.kind()) {
                return self.inner.handle(input.with_node(ancestor));
            }
            current = ancestor.parent();
        }
        None
    }
}
