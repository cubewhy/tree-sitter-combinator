# tree-sitter-utils

A composable, parser-combinator-style abstraction over tree-sitter node
dispatch. Instead of writing ad-hoc `loop { match node.kind() { ... } }`
patterns, you build a chain of typed, zero-cost `Handler` combinators that
express "how to map a syntax-tree node (plus arbitrary context) to an output
value". The crate is fully language-agnostic; all grammar knowledge lives in
the consumer crate.

---

## Quick start

Add the dependency:

```toml
[dependencies]
tree-sitter-utils = "0.1"
tree-sitter = "0.26"
# plus your grammar crate, e.g. tree-sitter-python = "0.25"
```

Build a handler chain for a fictional language with four node kinds:

```rust
use tree_sitter_utils::{handler_fn, never, HandlerExt, Input};

// Context your consumer crate supplies to every handler.
struct MyCtx<'a> {
    source: &'a str,
}

// Label nodes from a fictional grammar.
fn make_labeller<'a>() -> impl tree_sitter_utils::Handler<&'a MyCtx<'a>, String> {
    // 1. Handle "func_decl" and "lambda" the same way.
    let callable = handler_fn(|_: Input<&MyCtx<'_>>| "callable".to_owned())
        .for_kinds(&["func_decl", "lambda"]);

    // 2. Handle "identifier" only when it sits inside a call expression.
    let ident_in_call = handler_fn(|inp: Input<&MyCtx<'_>>| {
        format!("ident-in-call:{}", inp.node.kind())
    })
    .for_kinds(&["identifier"])
    .when(tree_sitter_utils::has_parent_kind("call_expr"));

    // 3. Climb to the nearest enclosing "block" for anything else.
    let block_climber = (|inp: Input<&MyCtx<'_>>| -> Option<String> {
        (inp.node.kind() == "block").then(|| "inside block".to_owned())
    })
    .climb(&["module"]);

    // Chain: try each in order, fall back to "unknown".
    callable
        .or(ident_in_call)
        .or(block_climber)
        .or(handler_fn(|inp: Input<&MyCtx<'_>>| {
            format!("unknown:{}", inp.node.kind())
        }))
}
```

---

## Illustrative example — Java-specific code lives in the consumer crate, not here

The original motivation was eliminating patterns like:

```java
// Java consumer crate (NOT part of tree-sitter-utils)
private String determineLocation(Node node, Context ctx) {
    while (node != null) {
        switch (node.getType()) {
            case "method_declaration":
                return labelMethod(node, ctx);
            case "class_declaration":
                return labelClass(node, ctx);
            case "lambda_expression":
                return "lambda";
        }
        node = node.getParent();
    }
    return null;
}
```

With `tree-sitter-utils` the same logic in the Java consumer crate collapses to:

```rust
// Java consumer crate — grammar strings stay here, NOT in tree-sitter-utils.
use tree_sitter_utils::{handler_fn, never, HandlerExt, Input};

fn make_location_handler() -> impl tree_sitter_utils::Handler<MyJavaCtx, String> {
    handler_fn(|inp: Input<MyJavaCtx>| label_method(&inp.node, &inp.ctx))
        .for_kinds(&["method_declaration"])
        .or(
            handler_fn(|inp: Input<MyJavaCtx>| label_class(&inp.node, &inp.ctx))
                .for_kinds(&["class_declaration"]),
        )
        .or(
            handler_fn(|_: Input<MyJavaCtx>| "lambda".to_owned())
                .for_kinds(&["lambda_expression"]),
        )
        .climb(&["program"]) // ascend until one of the above matches
}
// type MyJavaCtx = ();
// fn label_method(_: &tree_sitter::Node<'_>, _: &()) -> String { String::new() }
// fn label_class(_: &tree_sitter::Node<'_>, _: &()) -> String { String::new() }
```

The handler is built once, stored cheaply (no heap allocation in the hot
path), and called with a single `handler.handle(input)` per node.

---


## Traversal utilities

The `tree_sitter_utils::traversal` module provides a set of pure,
node-returning helpers for locating nodes in the syntax tree without going
through the combinator machinery. Use them when you need a `Node` value back
directly rather than a dispatched output.

### When to use traversal vs. combinators

| Goal | Tool |
|---|---|
| Find the nearest ancestor of a specific kind and use it as a `Node` | `traversal::ancestor_of_kind` / `ancestor_of_kinds` |
| Find the nearest ancestor of a specific kind, then run a handler on it | `.find_ancestor(kinds, stop)` combinator |
| Check whether **any** ancestor has a specific kind (as a predicate) | `has_ancestor_kind` / `has_ancestor_kinds` |
| Unwrap wrapper nodes to reach the inner expression | `traversal::peel_while_kind` |
| Find the first named child of a specific kind | `traversal::first_child_of_kind` / `first_child_of_kinds` |
| Find the first child (including anonymous tokens) of a specific kind | `traversal::any_child_of_kind` / `any_child_of_kinds` |
| Dispatch on node kind and produce a value | combinator chain (`handler_fn`, `.for_kinds`, `.or`, …) |

### Function reference

#### Ancestor traversal

```rust
use tree_sitter_utils::traversal::{ancestor_of_kind, ancestor_of_kinds};

// Walk up the tree and return the first strict ancestor whose kind matches.
// The starting node itself is never tested.
let method = ancestor_of_kind(identifier_node, "method_declaration");

// Multi-kind variant: return the first ancestor that matches any of the kinds.
let stmt = ancestor_of_kinds(node, &["break_statement", "continue_statement"]);
```

Both functions return `None` when the root is reached without a match.

#### Wrapper peeling

```rust
use tree_sitter_utils::traversal::peel_while_kind;

// Repeatedly descend into the first named child while the node kind is a
// wrapper kind.  Useful for stripping `parenthesized_expression` or
// `expression_statement` wrappers to reach the real expression inside.
let inner = peel_while_kind(node, &["parenthesized_expression"]);
```

If the node is not a wrapper kind it is returned unchanged. If a wrapper has
no named child, descending stops and the wrapper itself is returned.

#### Child search

```rust
use tree_sitter_utils::traversal::{
    first_child_of_kind, first_child_of_kinds,
    any_child_of_kind, any_child_of_kinds,
};

// First *named* child with a given kind (skips anonymous tokens).
let name = first_child_of_kind(decl_node, "identifier");

// First named child matching any of several kinds.
let param = first_child_of_kinds(node, &["formal_parameter", "spread_parameter"]);

// First child (named *or* anonymous) with a given kind.
// Use this for anonymous tokens like `new`, `.`, `;`.
let new_kw = any_child_of_kind(ctor_node, "new");

// Multi-kind variant of any_child_of_kind.
let op = any_child_of_kinds(binary_node, &["+", "-", "*", "/"]);
```

All child-search functions search **immediate children only** — they do not
perform deep traversal.

#### Offset-based search

```rust
use tree_sitter_utils::traversal::find_node_by_offset;

// Walk the subtree and return the innermost node of `kind` whose byte span
// contains `offset` (inclusive-start / exclusive-end, matching tree-sitter).
// When multiple nodes of that kind contain the offset (nested structures)
// the deepest one is returned.
let method = find_node_by_offset(root, "method_declaration", cursor_offset);
```

Returns `None` when no node of `kind` contains the offset.

### Traversal vs. `.find_ancestor` combinator

`traversal::ancestor_of_kind` and the `.find_ancestor` combinator solve
related but different problems:

```rust
use tree_sitter_utils::{handler_fn, HandlerExt, Input};
use tree_sitter_utils::traversal::ancestor_of_kind;

// traversal::ancestor_of_kind — you want the Node itself.
if let Some(method) = ancestor_of_kind(node, "method_declaration") {
    // work with `method` directly
}

// .find_ancestor combinator — you want to run a handler on the found ancestor.
let h = handler_fn(|inp: Input<()>| inp.node.kind().to_owned())
    .find_ancestor(&["method_declaration"], &["program"]);
// h.handle(input) finds the ancestor and passes it to the handler in one step.
```

---

## Query utilities

The `tree_sitter_utils::query` module wraps tree-sitter's
[`Query`](https://docs.rs/tree-sitter/latest/tree_sitter/struct.Query.html) /
[`QueryCursor`](https://docs.rs/tree-sitter/latest/tree_sitter/struct.QueryCursor.html)
API into two focused helpers that eliminate the streaming-iterator boilerplate
that every consumer crate otherwise duplicates.

### `run_query`

Execute a compiled query against a subtree and collect every match as a
`Vec<(capture_index, Node)>`.

```rust
use tree_sitter::{Query, Language};
use tree_sitter_utils::query::run_query;

# fn example(root: tree_sitter::Node, source: &[u8], lang: Language) {
let q = Query::new(&lang, "(identifier) @id").unwrap();

// Search the whole subtree.
let all_matches = run_query(&q, root, source, None);

// Restrict to a byte range (e.g. everything before the cursor).
let before_cursor = run_query(&q, root, source, Some(0..cursor_offset));
# }
```

Each element of the returned `Vec` is one match; within a match, captures
are `(capture_index, Node)` pairs in query-pattern order.

### `capture_text`

Extract the source text of one capture by its index from a single match.

```rust
use tree_sitter_utils::query::{run_query, capture_text};
use tree_sitter::{Query, Language};

# fn example(root: tree_sitter::Node, source: &[u8], lang: Language) {
let q = Query::new(&lang, "(identifier) @name").unwrap();
let name_idx = q.capture_index_for_name("name").unwrap();

for caps in run_query(&q, root, source, None) {
    if let Some(text) = capture_text(&caps, name_idx, source) {
        println!("{text}");
    }
}
# }
```

Returns `None` when the capture index is absent from the match or when the
node's bytes are not valid UTF-8.

## Combinator reference

| Combinator | Signature sketch | Semantics |
|---|---|---|
| `.or(other)` | `(H, H2) -> Or<H, H2>` | Try `self`; on `None`, try `other`. |
| `.when(pred)` | `(H, P: NodePredicate) -> When<H, P>` | Run `self` only when `pred` returns `true`. |
| `.for_kinds(kinds)` | `(H, &'static [&'static str]) -> When<H, KindIs>` | Sugar for `.when(kind_is(kinds))`. |
| `.map(f)` | `(H, Fn(R)->R2) -> Map<H, F, R>` | Transform a `Some(out)` result. |
| `.map_input(f)` | `(H, Fn(Input)->Input) -> MapInput<H, F>` | Transform the `Input` before passing it to `self`. |
| `.and_then(f)` | `(H, Fn(Input,R)->Option<R2>) -> AndThen<H,F,R>` | Flat-map: feed `(input, out)` into `f` on success. |
| `.climb(stop_kinds)` | `(H, &'static [&'static str]) -> Climb<H>` | On `None`, walk `parent()` and retry `self` until a stop-kind or root. |
| `.or_else_climb(other, stop_kinds)` | `(H, H2, &'static [&'static str]) -> OrElseClimb<H,H2>` | Try `self`; on `None`, try `other` on each ancestor up to stop-kind. |
| `.boxed()` | `H -> BoxedHandler<Ctx, R>` | Erase the type for dynamic dispatch (heap-allocates). |

Free-function constructors:

| Function | Returns | Semantics |
|---|---|---|
| `handler_fn(f)` | `HandlerFn<F>` | Wrap an infallible `Fn(Input)->R`; always returns `Some`. |
| `never()` | `Never<Ctx, R>` | Always returns `None`. |
| `always(value)` | `Always<R>` | Always returns `Some(value.clone())`. |
| `dispatch_on_kind(table)` | `DispatchOnKind<Ctx, R>` | Static kind→handler lookup table. |
| `first_of(handlers)` | `FirstOf<Ctx, R>` | Try a `Vec<BoxedHandler>` in order; return first `Some`. |
