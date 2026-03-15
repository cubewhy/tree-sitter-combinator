//! The [`Input`] type — a cheap, copyable snapshot passed to every handler.

/// A snapshot of the current tree-sitter node together with caller-supplied
/// context.
///
/// `Input` is designed to be passed by value; it is [`Copy`] whenever `Ctx`
/// is [`Copy`].
///
/// # Example
///
/// ```rust
/// // Input<()> uses the unit type as a zero-sized context.
/// // In real usage Ctx is typically a shared reference to your extractor.
/// use tree_sitter_combinator::Input;
/// fn accepts_input(_: Input<()>) {}
/// ```
#[derive(Clone, Copy)]
pub struct Input<'tree, Ctx> {
    /// The current syntax-tree node being examined.
    pub node: tree_sitter::Node<'tree>,
    /// Caller-supplied context (e.g. a reference to an extractor struct).
    pub ctx: Ctx,
    /// An optional trigger character hint (e.g. `.` or `(`), if the call
    /// site has one.
    pub trigger_char: Option<char>,
}

impl<'tree, Ctx> Input<'tree, Ctx> {
    /// Construct a new [`Input`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # /* doc-test is compile-only; we cannot build a real Node here */
    /// use tree_sitter_combinator::Input;
    /// // Input::new(node, ctx, None)
    /// ```
    #[inline]
    pub fn new(node: tree_sitter::Node<'tree>, ctx: Ctx, trigger_char: Option<char>) -> Self {
        Self { node, ctx, trigger_char }
    }

    /// Return a copy of this input with `node` replaced.
    ///
    /// Useful inside combinators that need to walk the tree.
    ///
    /// # Example
    ///
    /// ```rust
    /// # /* compile-only */
    /// use tree_sitter_combinator::Input;
    /// // let parent_input = input.with_node(parent_node);
    /// ```
    #[inline]
    pub fn with_node(self, node: tree_sitter::Node<'tree>) -> Self {
        Self { node, ..self }
    }
}
