//! Unit tests for individual combinators.

#[cfg(test)]
mod tests {
    use crate::{
        always, handler_fn, never,
        predicates::{has_parent_kind, kind_is, kind_is_not, node_depth_lte},
        Handler, HandlerExt, Input,
    };

    fn parse_python(src: &str) -> tree_sitter::Tree {
        let language: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language).unwrap();
        parser.parse(src, None).unwrap()
    }

    fn root_input(tree: &tree_sitter::Tree) -> Input<'_, ()> {
        Input::new(tree.root_node(), (), None)
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
    // handler_fn / never / always
    // -----------------------------------------------------------------------

    #[test]
    fn handler_fn_always_returns_some() {
        let (tree, h) = (parse_python("x = 1"), handler_fn(|_: Input<()>| 42u32));
        assert_eq!(h.handle(root_input(&tree)), Some(42));
    }

    #[test]
    fn never_always_returns_none() {
        let (tree, h) = (parse_python("x = 1"), never::<(), u32>());
        assert_eq!(h.handle(root_input(&tree)), None);
    }

    #[test]
    fn always_returns_cloned_value() {
        let (tree, h) = (
            parse_python("x = 1"),
            always::<(), String>("hello".to_owned()),
        );
        assert_eq!(h.handle(root_input(&tree)), Some("hello".to_owned()));
    }

    // -----------------------------------------------------------------------
    // .or
    // -----------------------------------------------------------------------

    #[test]
    fn or_returns_first_when_some() {
        let tree = parse_python("x = 1");
        // Use handler_fn so Ctx is inferred from the closure.
        let h = handler_fn(|_: Input<()>| 1u32).or(handler_fn(|_: Input<()>| 2u32));
        assert_eq!(h.handle(root_input(&tree)), Some(1));
    }

    #[test]
    fn or_falls_through_to_second() {
        let tree = parse_python("x = 1");
        let h = never::<(), u32>().or(handler_fn(|_: Input<()>| 99u32));
        assert_eq!(h.handle(root_input(&tree)), Some(99));
    }

    #[test]
    fn or_returns_none_when_both_fail() {
        let tree = parse_python("x = 1");
        let h = never::<(), u32>().or(never::<(), u32>());
        assert_eq!(h.handle(root_input(&tree)), None);
    }

    // -----------------------------------------------------------------------
    // .when / .for_kinds
    // -----------------------------------------------------------------------

    #[test]
    fn when_passes_on_true_predicate() {
        let tree = parse_python("x = 1");
        let h = handler_fn(|_: Input<()>| 7u32).when(kind_is(&["module"]));
        assert_eq!(h.handle(root_input(&tree)), Some(7));
    }

    #[test]
    fn when_blocks_on_false_predicate() {
        let tree = parse_python("x = 1");
        let h = handler_fn(|_: Input<()>| 7u32).when(kind_is(&["identifier"]));
        assert_eq!(h.handle(root_input(&tree)), None);
    }

    #[test]
    fn for_kinds_matches_correct_kind() {
        let tree = parse_python("x = 1");
        let h = handler_fn(|input: Input<()>| input.node.kind().to_owned()).for_kinds(&["module"]);
        assert_eq!(h.handle(root_input(&tree)), Some("module".to_owned()));
    }

    #[test]
    fn for_kinds_rejects_wrong_kind() {
        let tree = parse_python("x = 1");
        let h = handler_fn(|_: Input<()>| 1u32).for_kinds(&["identifier"]);
        assert_eq!(h.handle(root_input(&tree)), None);
    }

    // -----------------------------------------------------------------------
    // .map
    // -----------------------------------------------------------------------

    #[test]
    fn map_transforms_output() {
        let tree = parse_python("x = 1");
        let h = handler_fn(|_: Input<()>| 5u32).map(|n| n * 2);
        assert_eq!(h.handle(root_input(&tree)), Some(10));
    }

    #[test]
    fn map_propagates_none() {
        let tree = parse_python("x = 1");
        let h = never::<(), u32>().map(|n| n * 2);
        assert_eq!(h.handle(root_input(&tree)), None);
    }

    // -----------------------------------------------------------------------
    // .and_then
    // -----------------------------------------------------------------------

    #[test]
    fn and_then_chains_on_success() {
        let tree = parse_python("x = 1");
        let h =
            handler_fn(|_: Input<()>| 3u32).and_then(|_: Input<()>, n: u32| Some(n.to_string()));
        assert_eq!(h.handle(root_input(&tree)), Some("3".to_owned()));
    }

    #[test]
    fn and_then_can_fail() {
        let tree = parse_python("x = 1");
        let h = handler_fn(|_: Input<()>| 0u32)
            .and_then(|_: Input<()>, n: u32| if n > 0 { Some(n) } else { None });
        assert_eq!(h.handle(root_input(&tree)), None);
    }

    // -----------------------------------------------------------------------
    // .map_input
    // -----------------------------------------------------------------------

    #[test]
    fn map_input_transforms_trigger_char() {
        let tree = parse_python("x = 1");
        let h = handler_fn(|input: Input<()>| input.trigger_char).map_input(|mut i: Input<()>| {
            i.trigger_char = Some('.');
            i
        });
        assert_eq!(h.handle(root_input(&tree)), Some(Some('.')));
    }

    // -----------------------------------------------------------------------
    // .climb
    // -----------------------------------------------------------------------

    #[test]
    fn climb_finds_ancestor() {
        let src = "x = 1\n";
        let tree = parse_python(src);
        let root = tree.root_node();
        let ident = find_node(root, |n| n.kind() == "identifier").unwrap();
        let input = Input::new(ident, (), None);

        let h = (|inp: Input<()>| -> Option<String> {
            (inp.node.kind() == "module").then(|| "found module".to_owned())
        })
        .climb(&[]);
        assert_eq!(h.handle(input), Some("found module".to_owned()));
    }

    #[test]
    fn climb_stops_at_stop_kind() {
        let src = "x = 1\n";
        let tree = parse_python(src);
        let root = tree.root_node();
        let expr_stmt = root.child(0).unwrap();
        let input = Input::new(expr_stmt, (), None);
        let h = (|inp: Input<()>| -> Option<String> {
            (inp.node.kind() == "module").then(|| "module".to_owned())
        })
        .climb(&["expression_statement"]);
        assert_eq!(h.handle(input), None);
    }

    // -----------------------------------------------------------------------
    // Built-in predicates
    // -----------------------------------------------------------------------

    #[test]
    fn kind_is_not_predicate() {
        let tree = parse_python("x = 1");
        let h = handler_fn(|_: Input<()>| 1u32).when(kind_is_not(&["identifier"]));
        assert_eq!(h.handle(root_input(&tree)), Some(1));
    }

    #[test]
    fn has_parent_kind_predicate() {
        let src = "x = 1\n";
        let tree = parse_python(src);
        let root = tree.root_node();
        let expr_stmt = root.child(0).unwrap();
        let input = Input::new(expr_stmt, (), None);
        let h = handler_fn(|_: Input<()>| 1u32).when(has_parent_kind("module"));
        assert_eq!(h.handle(input), Some(1));
    }

    #[test]
    fn node_depth_lte_predicate() {
        let tree = parse_python("x = 1");
        let h = handler_fn(|_: Input<()>| 1u32).when(node_depth_lte(0));
        assert_eq!(h.handle(root_input(&tree)), Some(1));
    }

    #[test]
    fn node_depth_lte_rejects_deep_node() {
        let src = "x = 1\n";
        let tree = parse_python(src);
        let root = tree.root_node();
        let expr_stmt = root.child(0).unwrap();
        let input = Input::new(expr_stmt, (), None);
        let h = handler_fn(|_: Input<()>| 1u32).when(node_depth_lte(0));
        assert_eq!(h.handle(input), None);
    }
}
