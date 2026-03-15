//! The [`Handler`] trait and blanket implementations.

use crate::input::Input;

/// The result type returned by every handler — `Some(out)` on success,
/// `None` to signal "this handler does not apply here".
pub type HandlerResult<R> = Option<R>;

/// Core abstraction: maps an [`Input`] to an optional output value.
///
/// Implement this trait to create a custom handler, or use the
/// free-function constructors ([`crate::handler_fn`], [`crate::always`], …)
/// and the combinator extension methods provided by [`crate::HandlerExt`].
///
/// Lifetimes are on the `handle` method, not on the trait itself, so a
/// single handler instance can be reused across nodes from different trees.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{Handler, Input, HandlerResult};
///
/// struct MyHandler;
///
/// impl<Ctx: Copy> Handler<Ctx, String> for MyHandler {
///     fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<String> {
///         Some(input.node.kind().to_owned())
///     }
/// }
/// ```
pub trait Handler<Ctx, R>: Send + Sync {
    /// Attempt to produce an output value for the given input.
    ///
    /// Returns `None` when this handler does not apply.
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R>;
}

/// Blanket implementation: any `Fn(Input<'_, Ctx>) -> Option<R>` is a handler.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{Handler, Input};
///
/// fn use_handler<H: Handler<(), String>>(h: &H) { let _ = h; }
///
/// let f = |input: Input<()>| -> Option<String> {
///     Some(input.node.kind().to_owned())
/// };
/// use_handler(&f);
/// ```
impl<Ctx, R, F> Handler<Ctx, R> for F
where
    F: Fn(Input<'_, Ctx>) -> Option<R> + Send + Sync,
{
    #[inline]
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R> {
        self(input)
    }
}
