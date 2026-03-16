//! Tests for the three features added in the gap-fill pass:
//! - `has_ancestor_kind` predicate
//! - `FindAncestor` combinator (`.find_ancestor()`)
//! - `ForChildren` / `ScanChildren` combinators (`.for_children()` / `.scan_children()`)

#[cfg(test)]
mod tests {
    use crate::{
        handler_fn, never,
        predicates::has_ancestor_kind,
        Handler, HandlerExt, Input,
    };
    use crate::predicates::NodePredicate;

    fn parse_python(src: &str) -> tree_sitter::Tree {
        let language: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language).unwrap();
        parser.parse(src, None).unwrap()
    }

    /// Depth-first search for the first node matching `pred`.
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
    // has_ancestor_kind
    // -----------------------------------------------------------------------

    /// A deeply-nested identifier node should report that it has a "module"
    /// ancestor somewhere above it.
    #[test]
    fn has_ancestor_kind_finds_distant_ancestor() {
        // Python: `x = 1` — the identifier `x` is nested inside an
        // assignment which lives inside a module.
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let input = Input::new(ident, (), None);

        let pred = has_ancestor_kind("module");
        assert!(
            pred.test(input),
            "identifier should have a 'module' ancestor"
        );
    }

    /// The node itself is NOT tested — only strict ancestors.
    #[test]
    fn has_ancestor_kind_does_not_match_self() {
        let tree = parse_python("x = 1\n");
        // Use the module (root) node itself.
        let root = tree.root_node();
        assert_eq!(root.kind(), "module");
        let input = Input::new(root, (), None);

        let pred = has_ancestor_kind("module");
        assert!(
            !pred.test(input),
            "the node itself should not satisfy has_ancestor_kind"
        );
    }

    /// When no ancestor has the requested kind the predicate returns false.
    #[test]
    fn has_ancestor_kind_returns_false_when_absent() {
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let input = Input::new(ident, (), None);

        let pred = has_ancestor_kind("function_definition");
        assert!(
            !pred.test(input),
            "identifier has no function_definition ancestor in 'x = 1'"
        );
    }

    /// `has_ancestor_kind` used as a `.when()` guard correctly filters.
    #[test]
    fn has_ancestor_kind_as_when_guard() {
        // `def foo(): pass` — the identifier `foo` is inside a
        // function_definition, but `pass` is not.
        let src = "def foo(): pass\n";
        let tree = parse_python(src);

        let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned())
            .for_kinds(&["identifier"])
            .when(has_ancestor_kind("function_definition"));

        // `foo` identifier — lives inside function_definition.
        let foo = find_node(tree.root_node(), |n| {
            n.kind() == "identifier"
                && n.utf8_text(src.as_bytes()).ok() == Some("foo")
        })
        .unwrap();
        assert_eq!(
            h.handle(Input::new(foo, (), None)),
            Some("identifier".to_owned())
        );

        // The module root is an identifier-free node; confirm None.
        let root_input = Input::new(tree.root_node(), (), None);
        assert_eq!(h.handle(root_input), None);
    }

    // -----------------------------------------------------------------------
    // find_ancestor combinator
    // -----------------------------------------------------------------------

    /// Starting from a deeply nested node, `.find_ancestor` climbs to the
    /// requested ancestor kind and runs the handler there.
    #[test]
    fn find_ancestor_climbs_to_target_kind() {
        // `x = 1` — start from the identifier, find the "assignment" ancestor.
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let input = Input::new(ident, (), None);

        // The handler reports the kind of the node it receives (should be "assignment").
        let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned())
            .find_ancestor(&["assignment"], &[]);

        assert_eq!(h.handle(input), Some("assignment".to_owned()));
    }

    /// `.find_ancestor` returns `None` when no ancestor of the requested
    /// kind exists between the start node and the root.
    #[test]
    fn find_ancestor_returns_none_when_kind_absent() {
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let input = Input::new(ident, (), None);

        let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned())
            .find_ancestor(&["function_definition"], &[]);

        assert_eq!(h.handle(input), None);
    }

    /// `.find_ancestor` stops and returns `None` when it hits a `stop_kinds`
    /// node before finding the target.
    #[test]
    fn find_ancestor_stops_at_stop_kind() {
        // `x = 1` — ask for "module" but stop at "assignment".
        // The identifier's parent is "assignment", which is a stop kind,
        // so we should never reach "module".
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let input = Input::new(ident, (), None);

        let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned())
            .find_ancestor(&["module"], &["assignment"]);

        assert_eq!(
            h.handle(input),
            None,
            "should have stopped at 'assignment' before reaching 'module'"
        );
    }

    /// `.find_ancestor` does NOT match the original node itself.
    #[test]
    fn find_ancestor_does_not_match_self() {
        let tree = parse_python("x = 1\n");
        let module = tree.root_node();
        assert_eq!(module.kind(), "module");
        let input = Input::new(module, (), None);

        // Ask for "module" starting from the module itself — should return None
        // because the node itself is never tested.
        let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned())
            .find_ancestor(&["module"], &[]);

        assert_eq!(h.handle(input), None);
    }

    /// Contrast with `never().or_else_climb`: `.find_ancestor` runs the handler
    /// **once** on the found ancestor, not on every ancestor.
    #[test]
    fn find_ancestor_invokes_handler_on_ancestor_not_original() {
        let src = "def foo(): pass\n";
        let tree = parse_python(src);

        // Start from the `pass` statement's identifier node (if any) or
        // just use the pass_statement node directly.
        let pass_node =
            find_node(tree.root_node(), |n| n.kind() == "pass_statement").unwrap();
        let input = Input::new(pass_node, (), None);

        // Walk up to the function_definition and report the found node's kind.
        let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned())
            .find_ancestor(&["function_definition"], &[]);

        assert_eq!(h.handle(input), Some("function_definition".to_owned()));
    }

    // -----------------------------------------------------------------------
    // for_children / scan_children
    // -----------------------------------------------------------------------

    /// `.for_children()` collects the kinds of all named children.
    #[test]
    fn for_children_collects_all_matching_named_children() {
        // `x = 1` — the "assignment" node has named children: identifier + integer.
        let tree = parse_python("x = 1\n");
        let assignment =
            find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        let input = Input::new(assignment, (), None);

        // Collect the kinds of all named children.
        let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned()).for_children();

        let result = h.handle(input).unwrap();
        assert!(
            result.contains(&"identifier".to_owned()),
            "expected 'identifier' in children, got {:?}",
            result
        );
        assert!(
            result.contains(&"integer".to_owned()),
            "expected 'integer' in children, got {:?}",
            result
        );
    }

    /// `.for_children()` returns `Some(vec![])` on a leaf with no named children,
    /// never `None`.
    #[test]
    fn for_children_returns_empty_vec_for_leaf_node() {
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let input = Input::new(ident, (), None);

        let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned()).for_children();

        assert_eq!(h.handle(input), Some(vec![]));
    }

    /// `.for_children()` with a guarded inner handler only collects matching children.
    #[test]
    fn for_children_with_guard_collects_only_matching() {
        let tree = parse_python("x = 1\n");
        let assignment =
            find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        let input = Input::new(assignment, (), None);

        // Only collect children that are identifiers.
        let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned())
            .for_kinds(&["identifier"])
            .for_children();

        let result = h.handle(input).unwrap();
        assert_eq!(result, vec!["identifier".to_owned()]);
    }

    /// `.scan_children()` returns the first matching child and stops.
    #[test]
    fn scan_children_returns_first_match() {
        // `x = 1` — "assignment" has identifier then integer as named children.
        let tree = parse_python("x = 1\n");
        let assignment =
            find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        let input = Input::new(assignment, (), None);

        // Scan for the first named child, any kind.
        let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned()).scan_children();

        // Should return the kind of the first named child ("identifier").
        let result = h.handle(input);
        assert_eq!(result, Some("identifier".to_owned()));
    }

    /// `.scan_children()` returns `None` when no child satisfies the handler.
    #[test]
    fn scan_children_returns_none_when_no_match() {
        let tree = parse_python("x = 1\n");
        let assignment =
            find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        let input = Input::new(assignment, (), None);

        // A handler that only matches "function_definition" children — none exist.
        let h = (|inp: Input<()>| -> Option<String> {
            (inp.node.kind() == "function_definition")
                .then(|| inp.node.kind().to_owned())
        })
        .scan_children();

        assert_eq!(h.handle(input), None);
    }

    /// `.scan_children()` returns `None` for a leaf node (no named children).
    #[test]
    fn scan_children_returns_none_for_leaf() {
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let input = Input::new(ident, (), None);

        let h = never::<(), String>().scan_children();
        assert_eq!(h.handle(input), None);
    }
}
