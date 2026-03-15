//! [`NodePredicate`] trait and built-in predicate constructors.

use crate::input::Input;

/// A predicate over an [`Input`] — returns `true` when the node matches.
///
/// Any `F: Fn(Input<'_, Ctx>) -> bool + Send + Sync` implements this trait
/// automatically via the blanket implementation, so plain closures work too.
///
/// Use the free-function constructors ([`kind_is`], [`kind_is_not`], …) to
/// obtain named predicate values, or supply any compatible closure directly.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::{NodePredicate, kind_is};
///
/// fn accepts_pred<Ctx: Copy, P: NodePredicate<Ctx>>(_: P) {}
/// accepts_pred::<(), _>(kind_is(&["identifier"]));
/// accepts_pred::<(), _>(|input: tree_sitter_combinator::Input<()>| input.node.kind() == "identifier");
/// ```
pub trait NodePredicate<Ctx>: Send + Sync {
    /// Test whether the predicate holds for the given input.
    fn test(&self, input: Input<'_, Ctx>) -> bool;
}

/// Blanket impl: every `Fn(Input<'_, Ctx>) -> bool + Send + Sync` is a predicate.
impl<Ctx, F> NodePredicate<Ctx> for F
where
    F: Fn(Input<'_, Ctx>) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, input: Input<'_, Ctx>) -> bool {
        self(input)
    }
}

// ---------------------------------------------------------------------------
// Built-in predicate structs
// ---------------------------------------------------------------------------

/// Predicate: `true` when `node.kind()` is one of the given static `kinds`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::kind_is;
/// let pred = kind_is(&["identifier", "type_identifier"]);
/// let _ = pred;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct KindIs(pub &'static [&'static str]);

impl<Ctx> NodePredicate<Ctx> for KindIs {
    #[inline]
    fn test(&self, input: Input<'_, Ctx>) -> bool {
        self.0.contains(&input.node.kind())
    }
}

// SAFETY: &'static [&'static str] is inherently Send + Sync.
unsafe impl Send for KindIs {}
unsafe impl Sync for KindIs {}

/// Predicate: `true` when `node.kind()` is **not** in the given `kinds`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::kind_is_not;
/// let pred = kind_is_not(&["comment", "ERROR"]);
/// let _ = pred;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct KindIsNot(pub &'static [&'static str]);

impl<Ctx> NodePredicate<Ctx> for KindIsNot {
    #[inline]
    fn test(&self, input: Input<'_, Ctx>) -> bool {
        !self.0.contains(&input.node.kind())
    }
}

unsafe impl Send for KindIsNot {}
unsafe impl Sync for KindIsNot {}

/// Predicate: `true` when `node.parent()` has the given `kind`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::has_parent_kind;
/// let pred = has_parent_kind("function_definition");
/// let _ = pred;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct HasParentKind(pub &'static str);

impl<Ctx> NodePredicate<Ctx> for HasParentKind {
    #[inline]
    fn test(&self, input: Input<'_, Ctx>) -> bool {
        input.node.parent().is_some_and(|p| p.kind() == self.0)
    }
}

unsafe impl Send for HasParentKind {}
unsafe impl Sync for HasParentKind {}

/// Predicate: `true` when the node depth (root = 0) is at most `max`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::node_depth_lte;
/// let pred = node_depth_lte(3);
/// let _ = pred;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct NodeDepthLte(pub usize);

impl<Ctx> NodePredicate<Ctx> for NodeDepthLte {
    #[inline]
    fn test(&self, input: Input<'_, Ctx>) -> bool {
        let mut depth = 0usize;
        let mut current = input.node;
        while let Some(parent) = current.parent() {
            depth += 1;
            if depth > self.0 {
                return false;
            }
            current = parent;
        }
        depth <= self.0
    }
}

unsafe impl Send for NodeDepthLte {}
unsafe impl Sync for NodeDepthLte {}

// ---------------------------------------------------------------------------
// Public constructor functions
// ---------------------------------------------------------------------------

/// Returns a predicate that is `true` when `node.kind()` is in `kinds`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::kind_is;
/// let _ = kind_is(&["identifier"]);
/// ```
#[inline]
pub fn kind_is(kinds: &'static [&'static str]) -> KindIs {
    KindIs(kinds)
}

/// Returns a predicate that is `true` when `node.kind()` is **not** in `kinds`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::kind_is_not;
/// let _ = kind_is_not(&["comment", "ERROR"]);
/// ```
#[inline]
pub fn kind_is_not(kinds: &'static [&'static str]) -> KindIsNot {
    KindIsNot(kinds)
}

/// Returns a predicate that is `true` when the node's parent has `kind`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::has_parent_kind;
/// let _ = has_parent_kind("call_expression");
/// ```
#[inline]
pub fn has_parent_kind(kind: &'static str) -> HasParentKind {
    HasParentKind(kind)
}

/// Returns a predicate that is `true` when the node's tree-depth ≤ `max`.
///
/// # Example
///
/// ```rust
/// use tree_sitter_combinator::node_depth_lte;
/// let _ = node_depth_lte(5);
/// ```
#[inline]
pub fn node_depth_lte(max: usize) -> NodeDepthLte {
    NodeDepthLte(max)
}
