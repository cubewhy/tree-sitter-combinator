//! The [`Map`] combinator — transform a successful output value.

use crate::handler::{Handler, HandlerResult};
use crate::input::Input;
use std::marker::PhantomData;

/// A handler that applies `f` to the output of `inner` when it succeeds.
///
/// Constructed via [`HandlerExt::map`](crate::HandlerExt::map).
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::{handler_fn, HandlerExt};
///
/// let h = handler_fn(|_: tree_sitter_utils::Input<()>| 42u32)
///     .map(|n| n.to_string());
/// let _ = h;
/// ```
pub struct Map<H, F, R> {
    /// The source handler.
    pub inner: H,
    /// The mapping function.
    pub f: F,
    pub(crate) _marker: PhantomData<fn(R)>,
}

impl<Ctx, R, R2, H, F> Handler<Ctx, R2> for Map<H, F, R>
where
    H: Handler<Ctx, R>,
    F: Fn(R) -> R2 + Send + Sync,
    Ctx: Copy,
{
    #[inline]
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R2> {
        self.inner.handle(input).map(|out| (self.f)(out))
    }
}
