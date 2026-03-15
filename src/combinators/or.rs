//! The [`Or`] combinator — try the first handler, fall back to the second.

use crate::handler::{Handler, HandlerResult};
use crate::input::Input;

/// A handler that tries `A` first; if it returns `None`, tries `B`.
///
/// Constructed via [`HandlerExt::or`](crate::HandlerExt::or).
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{handler_fn, HandlerExt, never};
///
/// let h = never::<(), String>()
///     .or(handler_fn(|_: tree_sitter_combinator::Input<()>| "fallback".to_owned()));
/// let _ = h;
/// ```
pub struct Or<A, B> {
    /// Primary handler.
    pub first: A,
    /// Fallback handler.
    pub second: B,
}

impl<Ctx, R, A, B> Handler<Ctx, R> for Or<A, B>
where
    A: Handler<Ctx, R>,
    B: Handler<Ctx, R>,
    Ctx: Copy,
{
    #[inline]
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R> {
        self.first.handle(input).or_else(|| self.second.handle(input))
    }
}
