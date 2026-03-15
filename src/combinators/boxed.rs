//! The [`BoxedHandler`] type — type-erased dynamic dispatch.

use crate::handler::{Handler, HandlerResult};
use crate::input::Input;

/// Type alias for the inner boxed function used by [`BoxedHandler`].
type BoxFn<Ctx, R> =
    Box<dyn for<'tree> Fn(Input<'tree, Ctx>) -> HandlerResult<R> + Send + Sync>;

/// A type-erased handler that owns a `Box<dyn Fn(...)>`.
///
/// Use this only when you need to store handlers in a collection or return
/// them from a function with an erased type. In hot paths, prefer the
/// zero-cost combinator structs.
///
/// Constructed via [`HandlerExt::boxed`](crate::HandlerExt::boxed).
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{handler_fn, HandlerExt, BoxedHandler, Input};
///
/// let h: BoxedHandler<(), String> =
///     handler_fn(|_: Input<()>| "hello".to_owned()).boxed();
/// let _ = h;
/// ```
pub struct BoxedHandler<Ctx, R>(BoxFn<Ctx, R>);

impl<Ctx, R> BoxedHandler<Ctx, R> {
    /// Wrap a boxed closure as a [`BoxedHandler`].
    pub fn new<F>(f: F) -> Self
    where
        F: for<'tree> Fn(Input<'tree, Ctx>) -> HandlerResult<R> + Send + Sync + 'static,
    {
        BoxedHandler(Box::new(f))
    }
}

impl<Ctx, R> Handler<Ctx, R> for BoxedHandler<Ctx, R>
where
    Ctx: Copy,
{
    #[inline]
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R> {
        (self.0)(input)
    }
}
