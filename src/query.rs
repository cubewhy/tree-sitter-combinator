//! Query execution helpers.
//!
//! These utilities wrap the tree-sitter [`Query`] / [`QueryCursor`] API into
//! a small ergonomic layer that eliminates the boilerplate of setting up a
//! cursor, iterating with a [`StreamingIterator`], and collecting captures.
//!
//! # When to use this module
//!
//! | Goal | Tool |
//! |---|---|
//! | Run a query and collect all match captures | [`run_query`] |
//! | Extract text for a specific capture index from one match | [`capture_text`] |
//!
//! # Example
//!
//! ```rust,no_run
//! use tree_sitter::{Query, Language};
//! use tree_sitter_utils::query::{run_query, capture_text};
//!
//! # fn example(root: tree_sitter::Node, source: &[u8], lang: Language) {
//! let q = Query::new(&lang, "(identifier) @name").unwrap();
//! let matches = run_query(&q, root, source, None);
//! for caps in &matches {
//!     if let Some(text) = capture_text(caps, 0, source) {
//!         println!("{text}");
//!     }
//! }
//! # }
//! ```

use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

/// Execute `query` against `node` and return all capture groups for every match.
///
/// Each element of the returned `Vec` corresponds to one query match and
/// contains the captures for that match as `(capture_index, Node)` pairs.
/// Capture order within each match follows the order of `@name` patterns in
/// the query.
///
/// # Parameters
///
/// - `query` – A compiled [`Query`].
/// - `node` – The subtree root to run the query against.
/// - `source` – Source bytes of the file (required by tree-sitter's cursor).
/// - `byte_range` – Optional byte range to restrict the search.
///   Pass `None` to search the entire subtree.
///
/// # Example
///
/// ```rust,no_run
/// use tree_sitter::{Query, Language};
/// use tree_sitter_utils::query::run_query;
///
/// # fn example(root: tree_sitter::Node, source: &[u8], lang: Language) {
/// let q = Query::new(&lang, "(identifier) @id").unwrap();
/// let all = run_query(&q, root, source, None);
/// println!("found {} identifier matches", all.len());
/// # }
/// ```
pub fn run_query<'tree>(
    query: &Query,
    node: Node<'tree>,
    source: &[u8],
    byte_range: Option<std::ops::Range<usize>>,
) -> Vec<Vec<(u32, Node<'tree>)>> {
    let mut cursor = QueryCursor::new();
    if let Some(range) = byte_range {
        cursor.set_byte_range(range);
    }
    let mut results = Vec::new();
    let mut matches = cursor.matches(query, node, source);
    while let Some(m) = matches.next() {
        let captures: Vec<(u32, Node<'tree>)> =
            m.captures.iter().map(|c| (c.index, c.node)).collect();
        results.push(captures);
    }
    results
}

/// Look up the text of a capture by its index within one match's capture list.
///
/// Returns `None` if no capture with `index` is present, or if the node's
/// bytes are not valid UTF-8.
///
/// This is a convenience wrapper around the common pattern:
///
/// ```rust,ignore
/// captures.iter()
///     .find(|(idx, _)| *idx == index)
///     .and_then(|(_, node)| node.utf8_text(source).ok())
/// ```
///
/// # Parameters
///
/// - `captures` – One element from the outer `Vec` returned by [`run_query`].
/// - `index` – The capture index (matches the order of `@name` patterns;
///   use [`Query::capture_index_for_name`] to look it up by name).
/// - `source` – Source bytes of the file.
///
/// # Example
///
/// ```rust,no_run
/// use tree_sitter::{Query, Language};
/// use tree_sitter_utils::query::{run_query, capture_text};
///
/// # fn example(root: tree_sitter::Node, source: &[u8], lang: Language) {
/// let q = Query::new(&lang, "(identifier) @name").unwrap();
/// let matches = run_query(&q, root, source, None);
/// for caps in &matches {
///     if let Some(text) = capture_text(caps, 0, source) {
///         println!("identifier: {text}");
///     }
/// }
/// # }
/// ```
pub fn capture_text<'s>(
    captures: &[(u32, Node<'_>)],
    index: u32,
    source: &'s [u8],
) -> Option<&'s str> {
    captures
        .iter()
        .find(|(idx, _)| *idx == index)
        .and_then(|(_, node)| node.utf8_text(source).ok())
}

#[cfg(test)]
mod tests {
    use super::{capture_text, run_query};
    use tree_sitter::{Parser, Query};

    fn parse_python(src: &str) -> tree_sitter::Tree {
        let lang: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&lang).unwrap();
        parser.parse(src, None).unwrap()
    }

    fn python_lang() -> tree_sitter::Language {
        tree_sitter_python::LANGUAGE.into()
    }

    #[test]
    fn run_query_returns_all_identifier_matches() {
        let src = b"x = y\n";
        let tree = parse_python(std::str::from_utf8(src).unwrap());
        let q = Query::new(&python_lang(), "(identifier) @id").unwrap();
        let matches = run_query(&q, tree.root_node(), src, None);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn run_query_byte_range_restricts_results() {
        let src = b"x = y + z\n";
        let tree = parse_python(std::str::from_utf8(src).unwrap());
        let q = Query::new(&python_lang(), "(identifier) @id").unwrap();
        let matches = run_query(&q, tree.root_node(), src, Some(0..2));
        assert_eq!(matches.len(), 1);
        let text = capture_text(&matches[0], 0, src).unwrap();
        assert_eq!(text, "x");
    }

    #[test]
    fn run_query_returns_empty_when_no_match() {
        let src = b"x = 1\n";
        let tree = parse_python(std::str::from_utf8(src).unwrap());
        let q = Query::new(&python_lang(), "(function_definition) @fn").unwrap();
        let matches = run_query(&q, tree.root_node(), src, None);
        assert!(matches.is_empty());
    }

    #[test]
    fn capture_text_returns_text_for_known_index() {
        let src = b"hello = 1\n";
        let tree = parse_python(std::str::from_utf8(src).unwrap());
        let q = Query::new(&python_lang(), "(identifier) @name").unwrap();
        let matches = run_query(&q, tree.root_node(), src, None);
        assert!(!matches.is_empty());
        let text = capture_text(&matches[0], 0, src);
        assert_eq!(text, Some("hello"));
    }

    #[test]
    fn capture_text_returns_none_for_unknown_index() {
        let src = b"hello = 1\n";
        let tree = parse_python(std::str::from_utf8(src).unwrap());
        let q = Query::new(&python_lang(), "(identifier) @name").unwrap();
        let matches = run_query(&q, tree.root_node(), src, None);
        assert!(capture_text(&matches[0], 99, src).is_none());
    }
}
