//! Combinator extension trait and sub-modules.
//!
//! Import [`HandlerExt`] to unlock all combinator methods on any [`Handler`].

pub mod and_then;
pub mod boxed;
pub mod climb;
pub mod map;
pub mod or;
pub mod when;

use std::marker::PhantomData;

use crate::handler::Handler;
use crate::input::Input;
use crate::predicates::{kind_is, KindIs, NodePredicate};

use self::and_then::AndThen;
use self::boxed::BoxedHandler;
use self::climb::{Climb, OrElseClimb};
use self::map::Map;
use self::or::Or;
use self::when::When;

/// Extension trait that adds combinator methods to every [`Handler`].
///
/// This trait has a blanket implementation for all `T: Handler<Ctx, R>` and
/// is not intended to be used as a trait object.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{handler_fn, HandlerExt, Input, never};
///
/// let h = never::<(), u32>()
///     .or(handler_fn(|_: Input<()>| 42u32))
///     .map(|n| n.to_string());
/// let _ = h;
/// ```
pub trait HandlerExt<Ctx, R>: Handler<Ctx, R> + Sized {
    /// Try `self`; if it returns `None`, try `other`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tree_sitter_combinator::{never, handler_fn, HandlerExt, Input};
    ///
    /// let h = never::<(), String>()
    ///     .or(handler_fn(|_: Input<()>| "fallback".to_owned()));
    /// let _ = h;
    /// ```
    fn or<H: Handler<Ctx, R>>(self, other: H) -> Or<Self, H> {
        Or { first: self, second: other }
    }

    /// Run `self` only when `pred` returns `true`; otherwise return `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tree_sitter_combinator::{handler_fn, HandlerExt, kind_is, Input};
    ///
    /// let h = handler_fn(|_: Input<()>| "hit".to_owned())
    ///     .when(kind_is(&["identifier"]));
    /// let _ = h;
    /// ```
    fn when<P: NodePredicate<Ctx>>(self, pred: P) -> When<Self, P> {
        When { inner: self, pred }
    }

    /// Sugar for `.when(kind_is(kinds))` — only handle specific node kinds.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tree_sitter_combinator::{handler_fn, HandlerExt, Input};
    ///
    /// let h = handler_fn(|_: Input<()>| "ident".to_owned())
    ///     .for_kinds(&["identifier", "type_identifier"]);
    /// let _ = h;
    /// ```
    fn for_kinds(self, kinds: &'static [&'static str]) -> When<Self, KindIs> {
        self.when(kind_is(kinds))
    }

    /// Apply `f` to the output value when `self` succeeds.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tree_sitter_combinator::{handler_fn, HandlerExt, Input};
    ///
    /// let h = handler_fn(|_: Input<()>| 1u32).map(|n| n.to_string());
    /// let _ = h;
    /// ```
    fn map<F, R2>(self, f: F) -> Map<Self, F, R>
    where
        F: Fn(R) -> R2 + Send + Sync,
    {
        Map { inner: self, f, _marker: PhantomData }
    }

    /// Transform the [`Input`] before passing it to `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tree_sitter_combinator::{handler_fn, HandlerExt, Input};
    ///
    /// let h = handler_fn(|input: Input<()>| input.node.kind().to_owned())
    ///     .map_input(|mut i: Input<()>| { i.trigger_char = Some('.'); i });
    /// let _ = h;
    /// ```
    fn map_input<F>(self, f: F) -> MapInput<Self, F>
    where
        F: for<'tree> Fn(Input<'tree, Ctx>) -> Input<'tree, Ctx> + Send + Sync,
    {
        MapInput { inner: self, f }
    }

    /// On success, pass `(input, out)` to `f`, which may itself return `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tree_sitter_combinator::{handler_fn, HandlerExt, Input};
    ///
    /// let h = handler_fn(|_: Input<()>| 1u32)
    ///     .and_then(|_: Input<()>, n: u32| Some(n + 1));
    /// let _ = h;
    /// ```
    fn and_then<F, R2>(self, f: F) -> AndThen<Self, F, R>
    where
        F: Fn(Input<'_, Ctx>, R) -> Option<R2> + Send + Sync,
    {
        AndThen { inner: self, f, _marker: PhantomData }
    }

    /// Retry `self` on each ancestor until it succeeds, stopping at any kind
    /// in `stop_kinds` or at the tree root.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tree_sitter_combinator::{handler_fn, HandlerExt, Input};
    ///
    /// let h = handler_fn(|input: Input<()>| {
    ///     (input.node.kind() == "module").then(|| "module".to_owned())
    /// })
    /// .climb(&["source_file"]);
    /// let _ = h;
    /// ```
    fn climb(self, stop_kinds: &'static [&'static str]) -> Climb<Self> {
        Climb { inner: self, stop_kinds }
    }

    /// Try `self` on the original node; on `None`, try `other` on each ancestor.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tree_sitter_combinator::{never, HandlerExt, Input};
    ///
    /// let h = never::<(), String>()
    ///     .or_else_climb(
    ///         |input: tree_sitter_combinator::Input<()>| -> Option<String> {
    ///             Some(input.node.kind().to_owned())
    ///         },
    ///         &["source_file"],
    ///     );
    /// let _ = h;
    /// ```
    fn or_else_climb<O: Handler<Ctx, R>>(
        self,
        other: O,
        stop_kinds: &'static [&'static str],
    ) -> OrElseClimb<Self, O> {
        OrElseClimb { inner: self, other, stop_kinds }
    }

    /// Erase the concrete type into a [`BoxedHandler`] for dynamic dispatch.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tree_sitter_combinator::{handler_fn, HandlerExt, BoxedHandler, Input};
    ///
    /// let h: BoxedHandler<(), String> =
    ///     handler_fn(|_: Input<()>| "hi".to_owned()).boxed();
    /// let _ = h;
    /// ```
    fn boxed(self) -> BoxedHandler<Ctx, R>
    where
        Self: 'static,
        Ctx: Copy,
    {
        BoxedHandler::new(move |input| self.handle(input))
    }
}

/// Blanket implementation — every `Handler` gets all combinator methods.
impl<Ctx, R, T: Handler<Ctx, R>> HandlerExt<Ctx, R> for T {}

// ---------------------------------------------------------------------------
// MapInput combinator
// ---------------------------------------------------------------------------

/// A handler that transforms the [`Input`] before passing it to `inner`.
///
/// Constructed via [`HandlerExt::map_input`].
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{handler_fn, HandlerExt, Input};
///
/// let h = handler_fn(|input: Input<()>| input.node.kind().to_owned())
///     .map_input(|mut i: Input<()>| { i.trigger_char = Some('.'); i });
/// let _ = h;
/// ```
pub struct MapInput<H, F> {
    /// The inner handler.
    pub inner: H,
    /// The input-transformation function.
    pub f: F,
}

impl<Ctx, R, H, F> Handler<Ctx, R> for MapInput<H, F>
where
    H: Handler<Ctx, R>,
    F: for<'tree> Fn(Input<'tree, Ctx>) -> Input<'tree, Ctx> + Send + Sync,
    Ctx: Copy,
{
    #[inline]
    fn handle<'tree>(&self, input: Input<'tree, Ctx>) -> Option<R> {
        self.inner.handle((self.f)(input))
    }
}
