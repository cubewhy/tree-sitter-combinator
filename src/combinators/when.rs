//! The [`When`] combinator — guard a handler with a predicate.

use crate::handler::{Handler, HandlerResult};
use crate::input::Input;
use crate::predicates::NodePredicate;

/// A handler that runs `inner` only when `pred` returns `true`.
///
/// Constructed via [`HandlerExt::when`](crate::HandlerExt::when) or the
/// sugar [`HandlerExt::for_kinds`](crate::HandlerExt::for_kinds).
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{handler_fn, HandlerExt, kind_is};
///
/// let h = handler_fn(|_: tree_sitter_combinator::Input<()>| "hit".to_owned())
///     .when(kind_is(&["identifier"]));
/// let _ = h;
/// ```
pub struct When<H, P> {
    /// The guarded handler.
    pub inner: H,
    /// The predicate gate.
    pub pred: P,
}

impl<Ctx, R, H, P> Handler<Ctx, R> for When<H, P>
where
    H: Handler<Ctx, R>,
    P: NodePredicate<Ctx>,
    Ctx: Copy,
{
    #[inline]
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R> {
        if self.pred.test(input) {
            self.inner.handle(input)
        } else {
            None
        }
    }
}
