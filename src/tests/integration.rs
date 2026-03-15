//! End-to-end integration tests using `tree-sitter-python`.
//!
//! Covers:
//! 1. `.for_kinds` dispatching to the correct leaf handler.
//! 2. `.or` fallthrough when the first handler returns `None`.
//! 3. `.climb` ascending to a parent node and matching there.
//! 4. `.when` with a custom predicate returning `None` when pred is false.
//! 5. A chain of at least three combinators.

#[cfg(test)]
mod tests {
    use crate::{handler_fn, never, Handler, HandlerExt, Input};

    fn parse_python(src: &str) -> tree_sitter::Tree {
        let language: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language).unwrap();
        parser.parse(src, None).unwrap()
    }

    fn find_node<'t, F: Fn(tree_sitter::Node<'_>) -> bool + Copy>(
        node: tree_sitter::Node<'t>,
        pred: F,
    ) -> Option<tree_sitter::Node<'t>> {
        if pred(node) {
            return Some(node);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i as u32) {
                if let Some(found) = find_node(child, pred) {
                    return Some(found);
                }
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // Scenario 1: .for_kinds dispatches to the correct handler
    // -----------------------------------------------------------------------
    #[test]
    fn for_kinds_dispatches_to_correct_handler() {
        let src = "def foo(): pass\n";
        let tree = parse_python(src);
        let ident_node = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let input = Input::new(ident_node, (), None);

        let h = handler_fn(|_: Input<()>| "got identifier".to_owned())
            .for_kinds(&["identifier", "type_identifier"]);

        assert_eq!(h.handle(input), Some("got identifier".to_owned()));
    }

    #[test]
    fn for_kinds_returns_none_for_non_matching_kind() {
        let src = "def foo(): pass\n";
        let tree = parse_python(src);
        let root = tree.root_node();
        let input = Input::new(root, (), None);

        let h = handler_fn(|_: Input<()>| "hit".to_owned()).for_kinds(&["identifier"]);

        assert_eq!(h.handle(input), None);
    }

    // -----------------------------------------------------------------------
    // Scenario 2: .or fallthrough
    // -----------------------------------------------------------------------
    #[test]
    fn or_falls_through_when_first_returns_none() {
        let src = "x = 1\n";
        let tree = parse_python(src);
        let root = tree.root_node(); // kind == "module"
        let input = Input::new(root, (), None);

        // Use plain Option-returning closures so no double-Option wrapping occurs.
        let h = (|inp: Input<()>| -> Option<String> {
            (inp.node.kind() == "identifier").then(|| "identifier hit".to_owned())
        })
        .or(|inp: Input<()>| -> Option<String> {
            (inp.node.kind() == "module").then(|| "module hit".to_owned())
        });

        assert_eq!(h.handle(input), Some("module hit".to_owned()));
    }

    // -----------------------------------------------------------------------
    // Scenario 3: .climb ascends to parent and matches there
    // -----------------------------------------------------------------------
    #[test]
    fn climb_ascends_and_matches_parent() {
        let src = "x = 1\n";
        let tree = parse_python(src);
        let root = tree.root_node();
        let ident_node = find_node(root, |n| n.kind() == "identifier").unwrap();
        let input = Input::new(ident_node, (), None);

        let h = (|inp: Input<()>| -> Option<String> {
            (inp.node.kind() == "module").then(|| "reached module".to_owned())
        })
        .climb(&[]);

        assert_eq!(h.handle(input), Some("reached module".to_owned()));
    }

    // -----------------------------------------------------------------------
    // Scenario 4: .when with custom predicate returns None when pred is false
    // -----------------------------------------------------------------------
    #[test]
    fn when_with_custom_predicate_returns_none_when_false() {
        let src = "x = 1\n";
        let tree = parse_python(src);
        let root = tree.root_node();
        let input = Input::new(root, (), None);

        let h = handler_fn(|_: Input<()>| "triggered".to_owned())
            .when(|inp: Input<()>| inp.trigger_char == Some('.'));

        // No trigger char -> None.
        assert_eq!(h.handle(input), None);

        // With trigger char -> Some.
        let input_dot = Input::new(root, (), Some('.'));
        assert_eq!(h.handle(input_dot), Some("triggered".to_owned()));
    }

    // -----------------------------------------------------------------------
    // Scenario 5: chain of three+ combinators
    // -----------------------------------------------------------------------
    #[test]
    fn three_combinator_chain() {
        // .for_kinds + .or + .map  (three combinators)
        let src = "x = 1\n";
        let tree = parse_python(src);
        let ident_node = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let input = Input::new(ident_node, (), None);

        let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned())
            .for_kinds(&["identifier"]) // combinator 1
            .or(never::<(), String>()) // combinator 2
            .map(|s: String| s.to_uppercase()); // combinator 3

        assert_eq!(h.handle(input), Some("IDENTIFIER".to_owned()));
    }

    #[test]
    fn five_combinator_chain_with_and_then_and_or() {
        // .for_kinds + .map + .and_then + .or + .map  (five combinators)
        let src = "def foo(): pass\n";
        let tree = parse_python(src);
        let root = tree.root_node();
        let ident_node = find_node(root, |n| n.kind() == "identifier").unwrap();
        let input = Input::new(ident_node, (), None);

        let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned())
            .for_kinds(&["identifier"]) // guard
            .map(|s: String| s.len()) // transform
            .and_then(|_: Input<()>, len: usize| {
                if len > 0 {
                    Some(format!("len={len}"))
                } else {
                    None
                }
            }) // flat-map
            .or(never::<(), String>()) // fallback
            .map(|s: String| s.to_uppercase()); // final transform

        assert_eq!(h.handle(input), Some("LEN=10".to_owned()));
    }
}
