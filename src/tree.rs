//! Trees, modeled on Lip Gloss's `tree` package.
//!
//! ```
//! use ratatui_lipgloss::tree::{self, Tree};
//!
//! let t = tree::root(".")
//!     .child("macOS")
//!     .child(
//!         Tree::new()
//!             .root("Linux")
//!             .child("NixOS")
//!             .child("Arch Linux (btw)"),
//!     )
//!     .child("BSD");
//! assert_eq!(
//!     t.to_string(),
//!     ".\n├── macOS\n├── Linux\n│   ├── NixOS\n│   └── Arch Linux (btw)\n└── BSD"
//! );
//! ```
//!
//! A [`Tree`] is also a ratatui [`Widget`], and a [`StatefulWidget`] with a
//! [`TreeState`] that tracks the selection, collapsed nodes and scrolling,
//! which makes it a complete interactive tree view (ratatui has none built
//! in).
//!
//! The [`list`](crate::list) module is built on the same machinery, so
//! lists and trees can be nested in each other.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position as CellPosition, Rect};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{StatefulWidget, Widget};

use crate::{Position, Style, join_horizontal, join_vertical, to_ansi_string, width};

/// A view of the visible siblings passed to enumerators, indenters and
/// style functions (Lip Gloss's `Children`).
#[derive(Clone, Debug)]
pub struct Children<'a> {
    nodes: Vec<&'a Node>,
}

impl<'a> Children<'a> {
    /// Number of visible siblings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether there are no visible siblings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The sibling at `index`.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&'a Node> {
        self.nodes.get(index).copied()
    }

    /// Whether `index` is the last visible sibling.
    #[must_use]
    pub fn is_last(&self, index: usize) -> bool {
        index + 1 >= self.nodes.len()
    }

    /// Iterates over the visible siblings.
    pub fn iter(&self) -> impl Iterator<Item = &'a Node> + '_ {
        self.nodes.iter().copied()
    }
}

/// Produces the prefix (enumerator) or continuation (indenter) for child
/// `index`.
pub type EnumeratorFn = Arc<dyn Fn(&Children<'_>, usize) -> String + Send + Sync>;
/// Produces the style for child `index`.
pub type StyleFn = Arc<dyn Fn(&Children<'_>, usize) -> Style + Send + Sync>;

/// `├──` for every child and `└──` for the last one.
#[must_use]
pub fn default_enumerator(children: &Children<'_>, index: usize) -> String {
    if children.is_last(index) {
        "└──"
    } else {
        "├──"
    }
    .to_string()
}

/// Like [`default_enumerator`] with a rounded last branch (`╰──`).
#[must_use]
pub fn rounded_enumerator(children: &Children<'_>, index: usize) -> String {
    if children.is_last(index) {
        "╰──"
    } else {
        "├──"
    }
    .to_string()
}

/// `│  ` below every child but the last, which gets spaces.
#[must_use]
pub fn default_indenter(children: &Children<'_>, index: usize) -> String {
    if children.is_last(index) {
        "   "
    } else {
        "│  "
    }
    .to_string()
}

/// How a tree (and its descendants without their own renderer) is drawn.
#[derive(Clone)]
pub(crate) struct Renderer {
    pub enumerator: EnumeratorFn,
    pub indenter: EnumeratorFn,
    pub enumerator_style: StyleFn,
    pub indenter_style: StyleFn,
    pub item_style: StyleFn,
    pub root_style: Style,
    pub width: u16,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            enumerator: Arc::new(default_enumerator),
            indenter: Arc::new(default_indenter),
            enumerator_style: Arc::new(|_, _| Style::new().padding_right(1)),
            indenter_style: Arc::new(|_, _| Style::new().padding_right(1)),
            item_style: Arc::new(|_, _| Style::new()),
            root_style: Style::new(),
            width: 0,
        }
    }
}

impl fmt::Debug for Renderer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Renderer")
            .field("root_style", &self.root_style)
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}

/// A node in a tree: a [`Leaf`] or a nested [`Tree`].
#[derive(Clone, Debug)]
pub enum Node {
    /// A node without children.
    Leaf(Leaf),
    /// A node with (possibly zero) children.
    Tree(Box<Tree>),
}

impl Node {
    /// The node's text.
    #[must_use]
    pub fn value(&self) -> &Text<'static> {
        match self {
            Self::Leaf(l) => &l.value,
            Self::Tree(t) => &t.value,
        }
    }

    /// The node's text as a plain string.
    #[must_use]
    pub fn value_string(&self) -> String {
        crate::to_plain_string(self.value())
    }

    /// Whether the node is hidden.
    #[must_use]
    pub fn is_hidden(&self) -> bool {
        match self {
            Self::Leaf(l) => l.hidden,
            Self::Tree(t) => t.hidden,
        }
    }

    /// The node's children (empty for leaves).
    #[must_use]
    pub fn children(&self) -> &[Node] {
        match self {
            Self::Leaf(_) => &[],
            Self::Tree(t) => &t.children,
        }
    }
}

/// A node without children.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Leaf {
    value: Text<'static>,
    hidden: bool,
}

impl Leaf {
    /// Creates a leaf.
    #[must_use]
    pub fn new(value: impl Into<Text<'static>>) -> Self {
        Self {
            value: value.into(),
            hidden: false,
        }
    }

    /// Hides or shows the leaf.
    #[must_use]
    pub fn hide(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// The leaf's text.
    #[must_use]
    pub fn value(&self) -> &Text<'static> {
        &self.value
    }

    /// Whether the leaf is hidden.
    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }
}

macro_rules! node_from_text {
    ($($t:ty),*) => {$(
        impl From<$t> for Node {
            fn from(v: $t) -> Self {
                Self::Leaf(Leaf::new(v))
            }
        }
    )*};
}
node_from_text!(String, Text<'static>, Line<'static>, Span<'static>);

impl From<&str> for Node {
    fn from(v: &str) -> Self {
        Self::Leaf(Leaf::new(v.to_string()))
    }
}

impl From<&String> for Node {
    fn from(v: &String) -> Self {
        Self::Leaf(Leaf::new(v.clone()))
    }
}

impl From<Leaf> for Node {
    fn from(v: Leaf) -> Self {
        Self::Leaf(v)
    }
}

impl From<Tree> for Node {
    fn from(v: Tree) -> Self {
        Self::Tree(Box::new(v))
    }
}

/// A tree of nodes, rendered with branch enumerators like `├──` / `└──`.
///
/// Build it with [`Tree::new`] / [`root`], add nodes with [`Tree::child`]
/// and style it with the enumerator / indenter / item / root style methods.
/// As in Lip Gloss, adding a tree *without a root* right after a leaf turns
/// that leaf into the subtree's root:
///
/// ```
/// use ratatui_lipgloss::tree::{self, Tree};
///
/// let t = tree::root("fruit").child("citrus").child(Tree::new().child("yuzu").child("lime"));
/// assert_eq!(t.to_string(), "fruit\n└── citrus\n    ├── yuzu\n    └── lime");
/// ```
///
/// Style setters apply to this tree and to descendants that don't define
/// their own styling (a descendant that sets any style uses its own
/// settings, starting from the defaults).
#[derive(Clone, Debug, Default)]
pub struct Tree {
    value: Text<'static>,
    children: Vec<Node>,
    hidden: bool,
    collapsed: bool,
    offset: (usize, usize),
    renderer: Option<Renderer>,
    highlight_style: Option<Style>,
}

/// Creates a tree with the given root text (Lip Gloss's `tree.Root`).
#[must_use]
pub fn root(value: impl Into<Text<'static>>) -> Tree {
    Tree::new().root(value)
}

/// A rendered line tagged with the path of the node it belongs to.
#[derive(Clone, Debug)]
pub(crate) struct TaggedLine {
    pub path: Vec<usize>,
    pub line: Line<'static>,
}

impl Tree {
    /// Creates an empty tree without a root.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the root text.
    #[must_use]
    pub fn root(mut self, value: impl Into<Text<'static>>) -> Self {
        self.value = value.into();
        self
    }

    /// Adds a child: a string / `Text` / `Line` / `Span`, a [`Leaf`], a
    /// [`Tree`] or a [`List`](crate::list::List).
    #[must_use]
    pub fn child(mut self, child: impl Into<Node>) -> Self {
        self.push(child.into());
        self
    }

    /// Adds several children.
    #[must_use]
    pub fn children<I, N>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = N>,
        N: Into<Node>,
    {
        for c in children {
            self.push(c.into());
        }
        self
    }

    pub(crate) fn push(&mut self, node: Node) {
        match node {
            Node::Tree(mut t) if t.value.lines.is_empty() || width(&t.value) == 0 => {
                match self.children.pop() {
                    Some(Node::Leaf(leaf)) => {
                        t.value = leaf.value;
                        self.children.push(Node::Tree(t));
                    }
                    Some(Node::Tree(mut parent)) => {
                        parent.children.append(&mut t.children);
                        self.children.push(Node::Tree(parent));
                    }
                    None => self.children.push(Node::Tree(t)),
                }
            }
            other => self.children.push(other),
        }
    }

    /// Hides or shows the whole tree.
    #[must_use]
    pub fn hide(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Collapses the tree: its root is drawn but its children are not.
    /// (For interactive collapsing use [`TreeState`].)
    #[must_use]
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Only shows children `start..len - end` (Lip Gloss's `Offset`).
    #[must_use]
    pub fn offset(mut self, start: usize, end: usize) -> Self {
        self.offset = (start, end);
        self
    }

    /// The root text.
    #[must_use]
    pub fn value(&self) -> &Text<'static> {
        &self.value
    }

    /// The direct children.
    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.children
    }

    /// Whether the tree is hidden.
    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// Whether the tree is collapsed.
    #[must_use]
    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn renderer_mut(&mut self) -> &mut Renderer {
        self.renderer.get_or_insert_with(Renderer::default)
    }

    /// Sets the enumerator (the branch drawn before each child), e.g.
    /// [`default_enumerator`], [`rounded_enumerator`] or a closure.
    #[must_use]
    pub fn enumerator<F>(mut self, f: F) -> Self
    where
        F: Fn(&Children<'_>, usize) -> String + Send + Sync + 'static,
    {
        self.renderer_mut().enumerator = Arc::new(f);
        self
    }

    /// Sets the indenter (drawn in front of a child's descendants and below
    /// multi-line children), e.g. [`default_indenter`].
    #[must_use]
    pub fn indenter<F>(mut self, f: F) -> Self
    where
        F: Fn(&Children<'_>, usize) -> String + Send + Sync + 'static,
    {
        self.renderer_mut().indenter = Arc::new(f);
        self
    }

    /// Sets one style for every enumerator (default: `padding_right(1)`).
    #[must_use]
    pub fn enumerator_style(self, style: Style) -> Self {
        self.enumerator_style_func(move |_, _| style.clone())
    }

    /// Sets a per-child enumerator style.
    #[must_use]
    pub fn enumerator_style_func<F>(mut self, f: F) -> Self
    where
        F: Fn(&Children<'_>, usize) -> Style + Send + Sync + 'static,
    {
        self.renderer_mut().enumerator_style = Arc::new(f);
        self
    }

    /// Sets one style for every indenter (default: `padding_right(1)`).
    #[must_use]
    pub fn indenter_style(self, style: Style) -> Self {
        self.indenter_style_func(move |_, _| style.clone())
    }

    /// Sets a per-child indenter style.
    #[must_use]
    pub fn indenter_style_func<F>(mut self, f: F) -> Self
    where
        F: Fn(&Children<'_>, usize) -> Style + Send + Sync + 'static,
    {
        self.renderer_mut().indenter_style = Arc::new(f);
        self
    }

    /// Sets one style for every item.
    #[must_use]
    pub fn item_style(self, style: Style) -> Self {
        self.item_style_func(move |_, _| style.clone())
    }

    /// Sets a per-child item style.
    #[must_use]
    pub fn item_style_func<F>(mut self, f: F) -> Self
    where
        F: Fn(&Children<'_>, usize) -> Style + Send + Sync + 'static,
    {
        self.renderer_mut().item_style = Arc::new(f);
        self
    }

    /// Sets the style of the root text.
    #[must_use]
    pub fn root_style(mut self, style: Style) -> Self {
        self.renderer_mut().root_style = style;
        self
    }

    /// Pads every line to at least `width` columns (using the item style).
    #[must_use]
    pub fn width(mut self, width: u16) -> Self {
        self.renderer_mut().width = width;
        self
    }

    /// Style patched onto the selected node's lines when rendered as a
    /// [`StatefulWidget`] (default: reversed).
    #[must_use]
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = Some(style);
        self
    }

    /// Renders the tree to a ratatui [`Text`].
    #[must_use]
    pub fn render(&self) -> Text<'static> {
        Text::from(
            self.tagged_lines(None)
                .into_iter()
                .map(|t| t.line)
                .collect::<Vec<_>>(),
        )
    }

    /// Renders the tree to a string with ANSI escape codes.
    #[must_use]
    pub fn render_ansi(&self) -> String {
        to_ansi_string(&self.render())
    }

    /// The paths of all visible nodes, in display order. A path is the list
    /// of child indices from the top (the root itself is `[]`).
    #[must_use]
    pub fn visible_paths(&self, state: Option<&TreeState>) -> Vec<Vec<usize>> {
        let mut out: Vec<Vec<usize>> = Vec::new();
        for t in self.tagged_lines(state) {
            if out.last() != Some(&t.path) {
                out.push(t.path);
            }
        }
        out
    }

    /// Returns the node at `path` (`[]` is the tree itself).
    #[must_use]
    pub fn node_at(&self, path: &[usize]) -> Option<NodeRef<'_>> {
        let mut current = NodeRef::Tree(self);
        for &i in path {
            current = match current.children().get(i)? {
                Node::Leaf(l) => NodeRef::Leaf(l),
                Node::Tree(t) => NodeRef::Tree(t),
            };
        }
        Some(current)
    }

    pub(crate) fn tagged_lines(&self, state: Option<&TreeState>) -> Vec<TaggedLine> {
        let default = Renderer::default();
        let renderer = self.renderer.as_ref().unwrap_or(&default);
        let mut out = Vec::new();
        let mut path = Vec::new();
        let collapsed = |p: &[usize]| state.is_some_and(|s| s.is_collapsed(p));
        render_node(
            self,
            renderer,
            true,
            &Line::default(),
            &mut path,
            &collapsed,
            &mut out,
        );
        out
    }
}

/// A borrowed node returned by [`Tree::node_at`].
#[derive(Clone, Copy, Debug)]
pub enum NodeRef<'a> {
    /// A leaf.
    Leaf(&'a Leaf),
    /// A (sub)tree.
    Tree(&'a Tree),
}

impl<'a> NodeRef<'a> {
    /// The node's text.
    #[must_use]
    pub fn value(&self) -> &'a Text<'static> {
        match self {
            Self::Leaf(l) => &l.value,
            Self::Tree(t) => &t.value,
        }
    }

    /// The node's children (empty for leaves).
    #[must_use]
    pub fn children(&self) -> &'a [Node] {
        match self {
            Self::Leaf(_) => &[],
            Self::Tree(t) => &t.children,
        }
    }
}

fn pad_line(line: &mut Line<'static>, target: u16, style: &Style) {
    let w = line.width();
    let target = usize::from(target);
    if target > w {
        line.spans
            .push(Span::styled(" ".repeat(target - w), style.to_ratatui()));
    }
}

fn render_node(
    tree: &Tree,
    r: &Renderer,
    is_root: bool,
    prefix: &Line<'static>,
    path: &mut Vec<usize>,
    collapsed: &dyn Fn(&[usize]) -> bool,
    out: &mut Vec<TaggedLine>,
) {
    if tree.hidden {
        return;
    }
    if is_root && width(&tree.value) > 0 {
        for mut line in r.root_style.render(tree.value.clone()).lines {
            pad_line(&mut line, r.width, &r.root_style);
            out.push(TaggedLine {
                path: path.clone(),
                line,
            });
        }
    }
    if tree.collapsed || collapsed(path) {
        return;
    }

    let (start, end) = tree.offset;
    let stop = tree.children.len().saturating_sub(end);
    let visible: Vec<(usize, &Node)> = tree
        .children
        .iter()
        .enumerate()
        .skip(start)
        .take(stop.saturating_sub(start))
        .filter(|(_, n)| !n.is_hidden())
        .collect();
    let children = Children {
        nodes: visible.iter().map(|(_, n)| *n).collect(),
    };

    let max_len = (0..children.len())
        .map(|i| width(&(r.enumerator_style)(&children, i).render((r.enumerator)(&children, i))))
        .max()
        .unwrap_or(0);

    for (i, (orig, child)) in visible.iter().enumerate() {
        let indent_style = (r.indenter_style)(&children, i);
        let enum_style = (r.enumerator_style)(&children, i);
        let item_style = (r.item_style)(&children, i);

        let indent = indent_style.render((r.indenter)(&children, i));
        let mut node_prefix = enum_style.render((r.enumerator)(&children, i));
        let enum_bg = match enum_style.get_background() {
            Some(bg) => Style::new().background(bg),
            None => Style::new(),
        };
        let pad = max_len.saturating_sub(width(&node_prefix));
        if pad > 0 {
            node_prefix = join_horizontal(
                Position::TOP,
                [enum_bg.render(" ".repeat(pad)), node_prefix],
            );
        }
        let item = item_style.render(child.value().clone());
        while item.lines.len() > node_prefix.lines.len() {
            node_prefix = join_vertical(Position::LEFT, [node_prefix, indent.clone()]);
        }
        let mut multi_prefix = Text::from(prefix.clone());
        while node_prefix.lines.len() > multi_prefix.lines.len() {
            multi_prefix.lines.push(prefix.clone());
        }
        let block = join_horizontal(Position::TOP, [multi_prefix, node_prefix, item]);

        path.push(*orig);
        for mut line in block.lines {
            pad_line(&mut line, r.width, &item_style);
            out.push(TaggedLine {
                path: path.clone(),
                line,
            });
        }
        if let Node::Tree(sub) = child
            && !sub.children.is_empty()
        {
            let renderer = sub.renderer.as_ref().unwrap_or(r);
            let mut next_prefix = prefix.clone();
            if let Some(first) = indent.lines.first() {
                next_prefix.spans.extend(first.spans.iter().cloned());
            }
            render_node(sub, renderer, false, &next_prefix, path, collapsed, out);
        }
        path.pop();
    }
}

impl fmt::Display for Tree {
    /// Writes the tree as plain text (no styling).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crate::to_plain_string(&self.render()))
    }
}

impl Widget for &Tree {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Widget::render(Tree::render(self), area, buf);
    }
}

/// Selection, collapsed nodes and scroll offset for an interactive
/// [`Tree`] rendered as a [`StatefulWidget`].
///
/// Nodes are identified by *paths*: child indices from the top of the tree
/// (`[]` is the root line, `[1, 0]` the first child of the second child).
///
/// ```
/// use ratatui::{buffer::Buffer, layout::Rect, widgets::StatefulWidget};
/// use ratatui_lipgloss::tree::{self, Tree, TreeState};
///
/// let t = tree::root("~").child("a").child(Tree::new().child("b")).child("c");
/// let mut state = TreeState::default();
/// state.select_next(&t); // the root
/// state.select_next(&t); // "a"
/// state.toggle_selected(); // collapse "a" (hides "b")
/// let mut buf = Buffer::empty(Rect::new(0, 0, 8, 3));
/// StatefulWidget::render(&t, buf.area, &mut buf, &mut state);
/// assert_eq!(state.selected(), Some(&[0][..]));
/// assert_eq!(state.path_at(2, 2), Some(vec![1]));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeState {
    selected: Option<Vec<usize>>,
    collapsed: BTreeSet<Vec<usize>>,
    offset: usize,
    rows: Vec<Vec<usize>>,
    area: Rect,
}

impl TreeState {
    /// Creates an empty state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The selected node's path.
    #[must_use]
    pub fn selected(&self) -> Option<&[usize]> {
        self.selected.as_deref()
    }

    /// Selects a node by path (or clears the selection).
    pub fn select(&mut self, path: Option<Vec<usize>>) {
        self.selected = path;
    }

    fn step(&mut self, tree: &Tree, forward: bool) {
        let paths = tree.visible_paths(Some(self));
        if paths.is_empty() {
            self.selected = None;
            return;
        }
        let pos = self
            .selected
            .as_ref()
            .and_then(|s| paths.iter().position(|p| p == s));
        let next = match (pos, forward) {
            (None, true) => 0,
            (None, false) => paths.len() - 1,
            (Some(i), true) => (i + 1).min(paths.len() - 1),
            (Some(i), false) => i.saturating_sub(1),
        };
        self.selected = Some(paths[next].clone());
    }

    /// Selects the next visible node (the first if nothing is selected).
    pub fn select_next(&mut self, tree: &Tree) {
        self.step(tree, true);
    }

    /// Selects the previous visible node (the last if nothing is selected).
    pub fn select_previous(&mut self, tree: &Tree) {
        self.step(tree, false);
    }

    /// Selects the first visible node.
    pub fn select_first(&mut self, tree: &Tree) {
        self.selected = tree.visible_paths(Some(self)).into_iter().next();
    }

    /// Selects the last visible node.
    pub fn select_last(&mut self, tree: &Tree) {
        self.selected = tree.visible_paths(Some(self)).pop();
    }

    /// Whether the node at `path` is collapsed.
    #[must_use]
    pub fn is_collapsed(&self, path: &[usize]) -> bool {
        self.collapsed.contains(path)
    }

    /// Collapses the node at `path`.
    pub fn collapse(&mut self, path: Vec<usize>) {
        self.collapsed.insert(path);
    }

    /// Expands the node at `path`.
    pub fn expand(&mut self, path: &[usize]) {
        self.collapsed.remove(path);
    }

    /// Toggles the node at `path`; returns `true` if it is now collapsed.
    pub fn toggle(&mut self, path: Vec<usize>) -> bool {
        if self.collapsed.remove(&path) {
            false
        } else {
            self.collapsed.insert(path);
            true
        }
    }

    /// Toggles the selected node, if any.
    pub fn toggle_selected(&mut self) {
        if let Some(p) = self.selected.clone() {
            self.toggle(p);
        }
    }

    /// The first rendered line (scroll offset).
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Sets the scroll offset (it is adjusted on render to keep the
    /// selection visible).
    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    /// The path of the node drawn at a screen position during the last
    /// render, for mouse handling.
    #[must_use]
    pub fn path_at(&self, column: u16, row: u16) -> Option<Vec<usize>> {
        if !self.area.contains(CellPosition::new(column, row)) {
            return None;
        }
        self.rows.get(usize::from(row - self.area.y)).cloned()
    }
}

impl StatefulWidget for &Tree {
    type State = TreeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TreeState) {
        let lines = self.tagged_lines(Some(state));
        let height = usize::from(area.height);
        if let Some(sel) = &state.selected {
            let first = lines.iter().position(|l| &l.path == sel);
            let last = lines.iter().rposition(|l| &l.path == sel);
            if let (Some(first), Some(last)) = (first, last) {
                if first < state.offset {
                    state.offset = first;
                } else if height > 0 && last >= state.offset + height {
                    state.offset = (last + 1 - height).min(first);
                }
            }
        }
        state.offset = state.offset.min(lines.len().saturating_sub(1));
        let highlight = self
            .highlight_style
            .clone()
            .unwrap_or_else(|| Style::new().reverse(true));
        state.area = area;
        state.rows.clear();
        for (row, tagged) in lines.iter().skip(state.offset).take(height).enumerate() {
            let y = area.y + u16::try_from(row).unwrap_or(u16::MAX);
            let line_area = Rect::new(area.x, y, area.width, 1);
            Widget::render(&tagged.line, line_area, buf);
            if state.selected.as_ref() == Some(&tagged.path) {
                buf.set_style(line_area, highlight.to_ratatui());
            }
            state.rows.push(tagged.path.clone());
        }
    }
}
