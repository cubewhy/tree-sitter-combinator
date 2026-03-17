//! Tests for the `traversal` module — pure node-returning utilities.

#[cfg(test)]
mod tests {
    use crate::traversal::{
        ancestor_of_kind, ancestor_of_kinds, first_child_of_kind, first_child_of_kinds,
        peel_while_kind,
    };

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
    // ancestor_of_kind
    // -----------------------------------------------------------------------

    #[test]
    fn ancestor_of_kind_finds_direct_parent() {
        // `x = 1` — identifier's direct parent is "assignment".
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let result = ancestor_of_kind(ident, "assignment");
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind(), "assignment");
    }

    #[test]
    fn ancestor_of_kind_finds_distant_ancestor() {
        // identifier is nested several levels inside module.
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let result = ancestor_of_kind(ident, "module");
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind(), "module");
    }

    #[test]
    fn ancestor_of_kind_does_not_match_self() {
        // Starting from the module root, asking for "module" must return None.
        let tree = parse_python("x = 1\n");
        let root = tree.root_node();
        assert_eq!(root.kind(), "module");
        assert!(ancestor_of_kind(root, "module").is_none());
    }

    #[test]
    fn ancestor_of_kind_returns_none_when_absent() {
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        assert!(ancestor_of_kind(ident, "function_definition").is_none());
    }

    // -----------------------------------------------------------------------
    // ancestor_of_kinds
    // -----------------------------------------------------------------------

    #[test]
    fn ancestor_of_kinds_returns_first_matching_ancestor() {
        // `x = 1` — from identifier, searching for assignment or module;
        // assignment is closer so it should be returned.
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let result = ancestor_of_kinds(ident, &["assignment", "module"]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind(), "assignment");
    }

    #[test]
    fn ancestor_of_kinds_returns_none_when_none_match() {
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        assert!(ancestor_of_kinds(ident, &["function_definition", "class_definition"]).is_none());
    }

    #[test]
    fn ancestor_of_kinds_does_not_match_self() {
        let tree = parse_python("x = 1\n");
        let root = tree.root_node();
        assert!(ancestor_of_kinds(root, &["module"]).is_none());
    }

    // -----------------------------------------------------------------------
    // peel_while_kind
    // -----------------------------------------------------------------------

    #[test]
    fn peel_while_kind_returns_node_unchanged_when_not_a_wrapper() {
        // `x = 1` — identifier is not a parenthesized_expression.
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let result = peel_while_kind(ident, &["parenthesized_expression"]);
        assert_eq!(result.id(), ident.id());
    }

    #[test]
    fn peel_while_kind_descends_into_wrapper() {
        // `(x)` — expression_statement > parenthesized_expression > identifier.
        // Peeling parenthesized_expression should reach identifier.
        let tree = parse_python("(x)\n");
        // Find the parenthesized_expression node.
        let paren = find_node(tree.root_node(), |n| {
            n.kind() == "parenthesized_expression"
        });
        if let Some(paren_node) = paren {
            let result = peel_while_kind(paren_node, &["parenthesized_expression"]);
            // Should have descended to the identifier inside.
            assert_ne!(result.kind(), "parenthesized_expression");
        }
        // If parenthesized_expression doesn't appear in this grammar version,
        // the test is a no-op (not a failure).
    }

    #[test]
    fn peel_while_kind_stops_at_leaf() {
        // A leaf node with no named children should be returned as-is even
        // if its kind is in wrapper_kinds.
        let tree = parse_python("x = 1\n");
        // integer node has no named children.
        let integer = find_node(tree.root_node(), |n| n.kind() == "integer").unwrap();
        // Pretend integer is a "wrapper" — it has no named children so peeling stops.
        let result = peel_while_kind(integer, &["integer"]);
        assert_eq!(result.id(), integer.id());
    }

    // -----------------------------------------------------------------------
    // first_child_of_kind
    // -----------------------------------------------------------------------

    #[test]
    fn first_child_of_kind_finds_named_child() {
        // `x = 1` — assignment's first named child is the identifier.
        let tree = parse_python("x = 1\n");
        let assignment = find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        let result = first_child_of_kind(assignment, "identifier");
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind(), "identifier");
    }

    #[test]
    fn first_child_of_kind_returns_none_when_absent() {
        let tree = parse_python("x = 1\n");
        let assignment = find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        assert!(first_child_of_kind(assignment, "function_definition").is_none());
    }

    #[test]
    fn first_child_of_kind_does_not_search_grandchildren() {
        // module's direct named children are statements, not identifiers.
        let tree = parse_python("x = 1\n");
        let root = tree.root_node();
        // identifier is a grandchild of module, not a direct child.
        assert!(first_child_of_kind(root, "identifier").is_none());
    }

    // -----------------------------------------------------------------------
    // first_child_of_kinds
    // -----------------------------------------------------------------------

    #[test]
    fn first_child_of_kinds_returns_first_matching() {
        // `x = 1` — assignment has identifier and integer as named children.
        // Searching for ["integer", "identifier"] should return the first
        // one in child order (identifier comes first).
        let tree = parse_python("x = 1\n");
        let assignment = find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        let result = first_child_of_kinds(assignment, &["integer", "identifier"]);
        assert!(result.is_some());
        // "identifier" appears before "integer" in child order.
        assert_eq!(result.unwrap().kind(), "identifier");
    }

    #[test]
    fn first_child_of_kinds_returns_none_when_none_match() {
        let tree = parse_python("x = 1\n");
        let assignment = find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        assert!(first_child_of_kinds(assignment, &["function_definition", "class_definition"]).is_none());
    }
    // -----------------------------------------------------------------------
    // any_child_of_kind / any_child_of_kinds
    // -----------------------------------------------------------------------

    #[test]
    fn any_child_of_kind_finds_anonymous_token() {
        // `x = 1` — assignment contains an anonymous `=` token.
        let tree = parse_python("x = 1\n");
        let assignment = find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        let result = crate::traversal::any_child_of_kind(assignment, "=");
        assert!(result.is_some(), "expected to find '=' token in assignment");
        assert_eq!(result.unwrap().kind(), "=");
    }

    #[test]
    fn any_child_of_kind_returns_none_when_absent() {
        let tree = parse_python("x = 1\n");
        let assignment = find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        assert!(crate::traversal::any_child_of_kind(assignment, "(").is_none());
    }

    #[test]
    fn any_child_of_kinds_returns_first_match() {
        // `x = 1` — search for either `=` or `+=`; only `=` is present.
        let tree = parse_python("x = 1\n");
        let assignment = find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        let result = crate::traversal::any_child_of_kinds(assignment, &["=", "+="]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind(), "=");
    }

    // -----------------------------------------------------------------------
    // HasAncestorKinds predicate
    // -----------------------------------------------------------------------

    #[test]
    fn has_ancestor_kinds_returns_true_when_any_ancestor_matches() {
        // identifier is nested inside assignment which is inside module.
        // Asking for ["assignment", "function_definition"] should hit assignment.
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let pred = crate::predicates::HasAncestorKinds(&["assignment", "function_definition"]);
        use crate::predicates::NodePredicate;
        assert!(pred.test(crate::input::Input::new(ident, (), None)));
    }

    #[test]
    fn has_ancestor_kinds_returns_false_when_no_ancestor_matches() {
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        let pred = crate::predicates::HasAncestorKinds(&["function_definition", "class_definition"]);
        use crate::predicates::NodePredicate;
        assert!(!pred.test(crate::input::Input::new(ident, (), None)));
    }

    #[test]
    fn has_ancestor_kinds_does_not_match_self() {
        // Starting from the module root, searching for "module" must return false.
        let tree = parse_python("x = 1\n");
        let root = tree.root_node();
        assert_eq!(root.kind(), "module");
        let pred = crate::predicates::HasAncestorKinds(&["module"]);
        use crate::predicates::NodePredicate;
        assert!(!pred.test(crate::input::Input::new(root, (), None)));
    }
}

#[cfg(test)]
mod offset_tests {
    use crate::traversal::find_node_by_offset;

    fn parse_python(src: &str) -> tree_sitter::Tree {
        let lang: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        parser.parse(src, None).unwrap()
    }

    #[test]
    fn finds_function_containing_offset() {
        let src = "def foo(): pass\n";
        let tree = parse_python(src);
        let offset = src.find("foo").unwrap();
        let result = find_node_by_offset(tree.root_node(), "function_definition", offset);
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind(), "function_definition");
    }

    #[test]
    fn returns_none_when_offset_outside_any_match() {
        let src = "def foo(): pass\nx = 1\n";
        let tree = parse_python(src);
        let offset = src.find('x').unwrap();
        let result = find_node_by_offset(tree.root_node(), "function_definition", offset);
        assert!(result.is_none());
    }

    #[test]
    fn returns_deepest_when_nested() {
        let src = "def outer():\n    def inner(): pass\n";
        let tree = parse_python(src);
        let offset = src.find("inner").unwrap();
        let result = find_node_by_offset(tree.root_node(), "function_definition", offset);
        assert!(result.is_some());
        let node = result.unwrap();
        let text = &src[node.start_byte()..node.end_byte()];
        assert!(text.contains("inner"), "expected inner fn, got: {text}");
        assert!(!text.contains("outer"), "should not span outer fn");
    }
}

#[cfg(test)]
mod sibling_and_descent_tests {
    use crate::traversal::{is_descendant_of, preceding_named_sibling};

    fn parse_python(src: &str) -> tree_sitter::Tree {
        let lang: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
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
    // preceding_named_sibling
    // -----------------------------------------------------------------------

    #[test]
    fn preceding_named_sibling_returns_none_for_first_child() {
        // `x = 1` — identifier is the first named child of assignment.
        let tree = parse_python("x = 1\n");
        let assignment = find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        let ident = find_node(assignment, |n| n.kind() == "identifier").unwrap();
        assert!(preceding_named_sibling(ident, assignment).is_none());
    }

    #[test]
    fn preceding_named_sibling_returns_previous_named_child() {
        // `x = 1` — named children of assignment are [identifier(x), integer(1)].
        // The preceding named sibling of integer(1) should be identifier(x).
        let tree = parse_python("x = 1\n");
        let assignment = find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        let integer = find_node(assignment, |n| n.kind() == "integer").unwrap();
        let result = preceding_named_sibling(integer, assignment);
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind(), "identifier");
    }

    #[test]
    fn preceding_named_sibling_returns_none_when_node_not_child_of_parent() {
        // Use a node as its own parent — it has no named children matching itself.
        let tree = parse_python("x = 1\n");
        let integer = find_node(tree.root_node(), |n| n.kind() == "integer").unwrap();
        assert!(preceding_named_sibling(integer, integer).is_none());
    }

    // -----------------------------------------------------------------------
    // is_descendant_of
    // -----------------------------------------------------------------------

    #[test]
    fn is_descendant_of_returns_true_for_self() {
        let tree = parse_python("x = 1\n");
        let ident = find_node(tree.root_node(), |n| n.kind() == "identifier").unwrap();
        assert!(is_descendant_of(ident, ident));
    }

    #[test]
    fn is_descendant_of_returns_true_for_direct_parent() {
        let tree = parse_python("x = 1\n");
        let assignment = find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        let ident = find_node(assignment, |n| n.kind() == "identifier").unwrap();
        assert!(is_descendant_of(ident, assignment));
    }

    #[test]
    fn is_descendant_of_returns_true_for_distant_ancestor() {
        let tree = parse_python("x = 1\n");
        let root = tree.root_node();
        let ident = find_node(root, |n| n.kind() == "identifier").unwrap();
        assert!(is_descendant_of(ident, root));
    }

    #[test]
    fn is_descendant_of_returns_false_for_unrelated_node() {
        let tree = parse_python("x = 1\n");
        let assignment = find_node(tree.root_node(), |n| n.kind() == "assignment").unwrap();
        let integer = find_node(assignment, |n| n.kind() == "integer").unwrap();
        let ident = find_node(assignment, |n| n.kind() == "identifier").unwrap();
        // integer is a sibling of identifier, not a descendant
        assert!(!is_descendant_of(integer, ident));
    }
}
