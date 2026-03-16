//! Pure node-returning traversal utilities.
//!
//! These functions complement the [`Handler`](crate::Handler)-based combinator
//! API for cases where you need to **locate** a specific tree-sitter node
//! rather than dispatch a value out of it.
//!
//! # When to use traversal vs. combinators
//!
//! | Goal | Tool |
//! |---|---|
//! | Locate an ancestor node, then use it as a plain `Node` | [`ancestor_of_kind`] / [`ancestor_of_kinds`] |
//! | Locate an ancestor node, then run a handler on it | [`HandlerExt::find_ancestor`](crate::HandlerExt::find_ancestor) |
//! | Repeatedly unwrap wrapper nodes to reach the inner expression | [`peel_while_kind`] |
//! | Find the first named child of a specific kind | [`first_child_of_kind`] / [`first_child_of_kinds`] |
//! | Dispatch on node kind and produce a value | [`HandlerExt`](crate::HandlerExt) combinator chain |

use tree_sitter::Node;

// ---------------------------------------------------------------------------
// Ancestor traversal
// ---------------------------------------------------------------------------

/// Walk up the ancestor chain and return the first strict ancestor whose
/// kind equals `kind`.
///
/// "Strict" means the starting node itself is **not** tested — only its
/// ancestors are examined.
///
/// Returns `None` when the root is reached without finding a match.
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::traversal::ancestor_of_kind;
/// // let method = ancestor_of_kind(identifier_node, "method_declaration");
/// ```
pub fn ancestor_of_kind<'tree>(mut node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    loop {
        node = node.parent()?;
        if node.kind() == kind {
            return Some(node);
        }
    }
}

/// Walk up the ancestor chain and return the first strict ancestor whose
/// kind is one of `kinds`.
///
/// Equivalent to calling [`ancestor_of_kind`] for each kind in order, but
/// in a single pass — the first ancestor that matches **any** of the given
/// kinds is returned.
///
/// Returns `None` when the root is reached without finding a match.
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::traversal::ancestor_of_kinds;
/// // let stmt = ancestor_of_kinds(node, &["break_statement", "continue_statement"]);
/// ```
pub fn ancestor_of_kinds<'tree>(
    mut node: Node<'tree>,
    kinds: &[&str],
) -> Option<Node<'tree>> {
    loop {
        node = node.parent()?;
        if kinds.contains(&node.kind()) {
            return Some(node);
        }
    }
}

// ---------------------------------------------------------------------------
// Wrapper peeling
// ---------------------------------------------------------------------------

/// Repeatedly descend into the first named child while the current node's
/// kind is in `wrapper_kinds`, returning the innermost non-wrapper node.
///
/// This is useful for unwrapping expressions like
/// `parenthesized_expression` or `expression_statement` to reach the
/// actual expression inside.
///
/// If the starting node is not a wrapper kind, it is returned unchanged.
/// If a wrapper has no named child, descending stops and the wrapper itself
/// is returned.
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::traversal::peel_while_kind;
/// // Unwrap ((x + 1)) to reach the binary_expression node.
/// // let inner = peel_while_kind(node, &["parenthesized_expression"]);
/// ```
pub fn peel_while_kind<'tree>(mut node: Node<'tree>, wrapper_kinds: &[&str]) -> Node<'tree> {
    loop {
        if !wrapper_kinds.contains(&node.kind()) {
            return node;
        }
        let mut walker = node.walk();
        match node.named_children(&mut walker).next() {
            Some(child) => node = child,
            None => return node,
        }
    }
}

// ---------------------------------------------------------------------------
// Child search
// ---------------------------------------------------------------------------

/// Return the first **named** child of `node` whose kind equals `kind`.
///
/// This replaces the common pattern:
/// ```rust,ignore
/// node.named_children(&mut walker).find(|n| n.kind() == "identifier")
/// ```
///
/// Only the immediate children are searched — this is not a deep traversal.
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::traversal::first_child_of_kind;
/// // let name_node = first_child_of_kind(decl_node, "identifier");
/// ```
pub fn first_child_of_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut walker = node.walk();
    node.named_children(&mut walker)
        .find(|child| child.kind() == kind)
}

/// Return the first **named** child of `node` whose kind is one of `kinds`.
///
/// This replaces:
/// ```rust,ignore
/// node.named_children(&mut walker)
///     .find(|n| n.kind() == "foo" || n.kind() == "bar")
/// ```
///
/// Only the immediate children are searched — this is not a deep traversal.
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::traversal::first_child_of_kinds;
/// // let param = first_child_of_kinds(node, &["formal_parameter", "spread_parameter"]);
/// ```
pub fn first_child_of_kinds<'tree>(node: Node<'tree>, kinds: &[&str]) -> Option<Node<'tree>> {
    let mut walker = node.walk();
    node.named_children(&mut walker)
        .find(|child| kinds.contains(&child.kind()))
}

/// Return the first child of `node` (including anonymous children) whose
/// kind equals `kind`.
///
/// Unlike [`first_child_of_kind`], this searches **all** children — both
/// named and anonymous. Use this when the target node may be an anonymous
/// token (e.g. `"new"`, `"."`, `";"`).
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::traversal::any_child_of_kind;
/// // Find the anonymous `new` keyword inside an object_creation_expression.
/// // let new_tok = any_child_of_kind(ctor_node, "new");
/// ```
pub fn any_child_of_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut walker = node.walk();
    node.children(&mut walker)
        .find(|child| child.kind() == kind)
}

/// Return the first child of `node` (including anonymous children) whose
/// kind is one of `kinds`.
///
/// Like [`any_child_of_kind`] but accepts multiple target kinds.
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::traversal::any_child_of_kinds;
/// // let op = any_child_of_kinds(binary_node, &["+", "-", "*", "/"]);
/// ```
pub fn any_child_of_kinds<'tree>(node: Node<'tree>, kinds: &[&str]) -> Option<Node<'tree>> {
    let mut walker = node.walk();
    node.children(&mut walker)
        .find(|child| kinds.contains(&child.kind()))
}

// ---------------------------------------------------------------------------
// Offset-based search
// ---------------------------------------------------------------------------

/// Walk the subtree rooted at `root` and return the innermost node whose kind
/// equals `kind` and whose byte span contains `offset`.
///
/// "Contains" uses inclusive-start / exclusive-end semantics
/// (`node.start_byte() <= offset < node.end_byte()`), matching tree-sitter's
/// own convention.
///
/// When multiple nodes of the target kind contain the offset (e.g. nested
/// structures), the **deepest** one is returned — the DFS traversal keeps
/// overwriting the result as it descends, so the last written value is the
/// innermost match.
///
/// Returns `None` when no node of `kind` contains the offset.
///
/// # Example
///
/// ```rust
/// use tree_sitter_utils::traversal::find_node_by_offset;
/// // let method = find_node_by_offset(root, "method_declaration", cursor_offset);
/// ```
pub fn find_node_by_offset<'tree>(
    root: Node<'tree>,
    kind: &str,
    offset: usize,
) -> Option<Node<'tree>> {
    fn dfs<'a>(node: Node<'a>, kind: &str, offset: usize, result: &mut Option<Node<'a>>) {
        if offset < node.start_byte() || offset >= node.end_byte() {
            return;
        }
        if node.kind() == kind {
            *result = Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            dfs(child, kind, offset, result);
        }
    }
    let mut result = None;
    dfs(root, kind, offset, &mut result);
    result
}
