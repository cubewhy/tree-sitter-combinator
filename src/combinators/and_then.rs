//! The [`AndThen`] combinator — flat-map on success.

use crate::handler::{Handler, HandlerResult};
use crate::input::Input;
use std::marker::PhantomData;

/// A handler that, on success, feeds `(input, out)` into a second function
/// that may itself fail.
///
/// Constructed via [`HandlerExt::and_then`](crate::HandlerExt::and_then).
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{handler_fn, HandlerExt, Input};
///
/// let h = handler_fn(|_: Input<()>| 42u32)
///     .and_then(|_input: Input<()>, n: u32| if n > 0 { Some(n.to_string()) } else { None });
/// let _ = h;
/// ```
pub struct AndThen<H, F, R> {
    /// The source handler.
    pub inner: H,
    /// The flat-mapping function.
    pub f: F,
    pub(crate) _marker: PhantomData<fn(R)>,
}

impl<Ctx, R, R2, H, F> Handler<Ctx, R2> for AndThen<H, F, R>
where
    H: Handler<Ctx, R>,
    F: Fn(Input<'_, Ctx>, R) -> HandlerResult<R2> + Send + Sync,
    Ctx: Copy,
{
    #[inline]
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R2> {
        self.inner.handle(input).and_then(|out| (self.f)(input, out))
    }
}
