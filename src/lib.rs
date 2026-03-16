//! # tree-sitter-utils
//!
//! A composable, parser-combinator-style abstraction over tree-sitter node
//! dispatch. Instead of writing ad-hoc `match node.kind() { ... }` loops,
//! you build a chain of typed, zero-cost [`Handler`] combinators that express
//! "how to map a syntax-tree node (plus arbitrary context) to an output value".
//!
//! The crate is fully language-agnostic. All grammar knowledge lives in the
//! consumer crate.
//!
//! ## Quick example
//!
//! ```rust
//! use tree_sitter_utils::{Input, handler_fn, HandlerExt};
//!
//! // A minimal handler that labels identifier nodes.
//! let h = handler_fn(|input: Input<()>| format!("node:{}", input.node.kind()))
//!     .for_kinds(&["identifier", "type_identifier"])
//!     .map(|s| s.to_uppercase());
//! # let _ = h;
//! ```

pub mod input;
pub mod handler;
pub mod predicates;
pub mod combinators;
pub mod constructors;
pub mod traversal;

#[cfg(test)]
mod tests;

pub use input::Input;
pub use handler::{Handler, HandlerResult};
pub use predicates::{NodePredicate, kind_is, kind_is_not, has_parent_kind, node_depth_lte};
pub use predicates::{has_ancestor_kind, HasAncestorKind};
pub use predicates::{has_ancestor_kinds, HasAncestorKinds};
pub use combinators::HandlerExt;
pub use combinators::boxed::BoxedHandler;
pub use combinators::find_ancestor::FindAncestor;
pub use combinators::for_children::{ForChildren, ScanChildren};
pub use constructors::{handler_fn, dispatch_on_kind, never, always, first_of};
