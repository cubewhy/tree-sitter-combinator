//! The [`Climb`] and [`OrElseClimb`] combinators — walk up the syntax tree.

use crate::handler::{Handler, HandlerResult};
use crate::input::Input;

/// A handler that retries itself on each ancestor node, stopping when it
/// succeeds, when it reaches a node whose kind is in `stop_kinds`, or when
/// there are no more parents.
///
/// The first attempt is made on the **original** node; climbing starts only
/// if that attempt returns `None`.
///
/// Constructed via [`HandlerExt::climb`](crate::HandlerExt::climb).
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::{handler_fn, HandlerExt, Input};
///
/// // Climbs until it finds a node whose kind is "module" or hits the root.
/// let h = (|input: tree_sitter_utils::Input<()>| -> Option<String> {
///     (input.node.kind() == "module").then(|| "found module".to_owned())
/// })
/// .climb(&["source_file"]);
/// let _ = h;
/// ```
pub struct Climb<H> {
    /// The handler tried at each level.
    pub inner: H,
    /// Kinds at which climbing stops (regardless of success).
    pub stop_kinds: &'static [&'static str],
}

impl<Ctx, R, H> Handler<Ctx, R> for Climb<H>
where
    H: Handler<Ctx, R>,
    Ctx: Copy,
{
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R> {
        let mut current = input.node;
        loop {
            let attempt = self.inner.handle(input.with_node(current));
            if attempt.is_some() {
                return attempt;
            }
            // Stop if the current node is a stop-kind.
            if self.stop_kinds.contains(&current.kind()) {
                return None;
            }
            match current.parent() {
                Some(parent) => current = parent,
                None => return None,
            }
        }
    }
}

/// A handler that first tries `inner` on the current node; if that returns
/// `None`, walks up the tree trying `other` at each ancestor, stopping at
/// any kind in `stop_kinds` or at the root.
///
/// Constructed via [`HandlerExt::or_else_climb`](crate::HandlerExt::or_else_climb).
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::{never, HandlerExt, Input};
///
/// let leaf = never::<(), String>();
/// let h = leaf.or_else_climb(
///     |inp: tree_sitter_utils::Input<()>| -> Option<String> {
///         (inp.node.kind() == "module").then(|| "module".to_owned())
///     },
///     &["source_file"],
/// );
/// let _ = h;
/// ```
pub struct OrElseClimb<H, O> {
    /// Handler tried first on the original node.
    pub inner: H,
    /// Handler tried on each ancestor when `inner` fails.
    pub other: O,
    /// Kinds at which climbing stops.
    pub stop_kinds: &'static [&'static str],
}

impl<Ctx, R, H, O> Handler<Ctx, R> for OrElseClimb<H, O>
where
    H: Handler<Ctx, R>,
    O: Handler<Ctx, R>,
    Ctx: Copy,
{
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R> {
        // First try `inner` on the original node.
        if let Some(out) = self.inner.handle(input) {
            return Some(out);
        }
        // Then climb with `other`.
        let mut current = input.node;
        loop {
            if self.stop_kinds.contains(&current.kind()) {
                return None;
            }
            match current.parent() {
                Some(parent) => {
                    current = parent;
                    let attempt = self.other.handle(input.with_node(current));
                    if attempt.is_some() {
                        return attempt;
                    }
                }
                None => return None,
            }
        }
    }
}
