//! Free-function handler constructors.

use crate::handler::{Handler, HandlerResult};
use crate::input::Input;

// ---------------------------------------------------------------------------
// handler_fn
// ---------------------------------------------------------------------------

/// Wrap an infallible function as a [`Handler`] that always returns `Some`.
///
/// Use this when your mapping function cannot fail.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{handler_fn, Handler, Input};
///
/// let h = handler_fn(|input: Input<()>| input.node.kind().to_owned());
/// let _ = h;
/// ```
pub fn handler_fn<Ctx, R, F>(f: F) -> HandlerFn<F>
where
    F: Fn(Input<'_, Ctx>) -> R + Send + Sync,
{
    HandlerFn(f)
}

/// Handler produced by [`handler_fn`].
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{handler_fn, Input};
/// let h = handler_fn(|_: Input<()>| 42u32);
/// let _ = h;
/// ```
pub struct HandlerFn<F>(F);

impl<Ctx, R, F> Handler<Ctx, R> for HandlerFn<F>
where
    F: Fn(Input<'_, Ctx>) -> R + Send + Sync,
    Ctx: Copy,
{
    #[inline]
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R> {
        Some((self.0)(input))
    }
}

// ---------------------------------------------------------------------------
// never / always
// ---------------------------------------------------------------------------

/// A handler that always returns `None`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{never, HandlerExt, handler_fn, Input};
///
/// let h = never::<(), String>()
///     .or(handler_fn(|_: Input<()>| "fallback".to_owned()));
/// let _ = h;
/// ```
pub fn never<Ctx, R>() -> Never<Ctx, R> {
    Never(std::marker::PhantomData)
}

/// Handler produced by [`never()`].
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::never;
/// let h = never::<(), u32>();
/// let _ = h;
/// ```
pub struct Never<Ctx, R>(std::marker::PhantomData<fn(Ctx) -> R>);

impl<Ctx, R> Handler<Ctx, R> for Never<Ctx, R> {
    #[inline]
    fn handle<'tree>(&self, _input: Input<'tree, Ctx>) -> HandlerResult<R> {
        None
    }
}

/// A handler that always returns `Some(value.clone())`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::always;
/// let h = always::<(), _>("hello".to_owned());
/// let _ = h;
/// ```
pub fn always<Ctx, R: Clone + Send + Sync>(value: R) -> Always<R> {
    Always(value)
}

/// Handler produced by [`always`].
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::always;
/// let h = always::<(), u32>(42);
/// let _ = h;
/// ```
pub struct Always<R>(pub R);

impl<R> Always<R> {
    /// Construct an [`Always`] handler in a `const` / `static` context.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tree_sitter_combinator::constructors::Always;
    /// static H: Always<u32> = Always::new_const(42u32);
    /// ```
    pub const fn new_const(value: R) -> Self {
        Always(value)
    }
}

impl<Ctx, R: Clone + Send + Sync> Handler<Ctx, R> for Always<R> {
    #[inline]
    fn handle<'tree>(&self, _input: Input<'tree, Ctx>) -> HandlerResult<R> {
        Some(self.0.clone())
    }
}

// ---------------------------------------------------------------------------
// dispatch_on_kind
// ---------------------------------------------------------------------------

/// Build a handler that dispatches on `node.kind()` via a static lookup table.
///
/// Entries are tried in order; the first matching kind wins.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::dispatch_on_kind;
/// // let h = dispatch_on_kind(&[("identifier", &my_handler)]);
/// let _ = dispatch_on_kind::<(), u32>(&[]);
/// ```
pub fn dispatch_on_kind<Ctx: 'static, R: 'static>(
    table: &'static [(&'static str, &'static dyn Handler<Ctx, R>)],
) -> DispatchOnKind<Ctx, R> {
    DispatchOnKind { table }
}

/// Handler produced by [`dispatch_on_kind`].
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::dispatch_on_kind;
/// let h = dispatch_on_kind::<(), u32>(&[]);
/// let _ = h;
/// ```
pub struct DispatchOnKind<Ctx: 'static, R: 'static> {
    /// The static dispatch table.
    pub table: &'static [(&'static str, &'static dyn Handler<Ctx, R>)],
}

impl<Ctx, R> Handler<Ctx, R> for DispatchOnKind<Ctx, R>
where
    Ctx: Copy + 'static,
    R: 'static,
{
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R> {
        let kind = input.node.kind();
        for (k, h) in self.table {
            if *k == kind {
                return h.handle(input);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// first_of
// ---------------------------------------------------------------------------

/// Try a slice of boxed handlers in order; return the first `Some` result.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{first_of, always, never, BoxedHandler, HandlerExt};
///
/// let handlers: Vec<BoxedHandler<(), u32>> = vec![
///     never::<(), u32>().boxed(),
///     always::<(), u32>(1).boxed(),
/// ];
/// let h = first_of(handlers);
/// let _ = h;
/// ```
pub fn first_of<Ctx, R>(
    handlers: Vec<crate::combinators::boxed::BoxedHandler<Ctx, R>>,
) -> FirstOf<Ctx, R> {
    FirstOf { handlers }
}

/// Handler produced by [`first_of`].
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::first_of;
/// let h = first_of::<(), u32>(vec![]);
/// let _ = h;
/// ```
pub struct FirstOf<Ctx, R> {
    handlers: Vec<crate::combinators::boxed::BoxedHandler<Ctx, R>>,
}

impl<Ctx, R> Handler<Ctx, R> for FirstOf<Ctx, R>
where
    Ctx: Copy,
{
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> HandlerResult<R> {
        for h in &self.handlers {
            let result = h.handle(input);
            if result.is_some() {
                return result;
            }
        }
        None
    }
}
