//! Lists, modeled on Lip Gloss's `list` package.
//!
//! ```
//! use ratatui_lipgloss::list::{self, List};
//!
//! let l = List::new(["A", "B", "C"]);
//! assert_eq!(l.to_string(), "• A\n• B\n• C");
//!
//! // Nested lists attach to the item before them.
//! let l = List::new(["Fruit"])
//!     .item(List::new(["Apple", "Yuzu"]).enumerator(list::roman))
//!     .item("Vegetables");
//! assert_eq!(l.to_string(), "• Fruit\n   I. Apple\n  II. Yuzu\n• Vegetables");
//! ```
//!
//! A list is a [`Tree`] without a root and with list-style enumerators, so
//! lists and trees can be nested in one another, and a list can be rendered
//! interactively with a [`TreeState`].

use std::fmt;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::{StatefulWidget, Widget};

use crate::Style;
use crate::tree::{Children, Node, Tree, TreeState};

/// The items passed to list enumerators and style functions.
pub type Items<'a> = Children<'a>;

/// `•`
#[must_use]
pub fn bullet(_: &Items<'_>, _: usize) -> String {
    "•".to_string()
}

/// `*`
#[must_use]
pub fn asterisk(_: &Items<'_>, _: usize) -> String {
    "*".to_string()
}

/// `-`
#[must_use]
pub fn dash(_: &Items<'_>, _: usize) -> String {
    "-".to_string()
}

/// `1.`, `2.`, `3.`, …
#[must_use]
pub fn arabic(_: &Items<'_>, i: usize) -> String {
    format!("{}.", i + 1)
}

/// `A.`, `B.`, …, `Z.`, `AA.`, `AB.`, …
#[must_use]
pub fn alphabet(_: &Items<'_>, i: usize) -> String {
    const N: usize = 26;
    let letter = |n: usize| char::from(b'A' + u8::try_from(n % N).unwrap_or(0));
    if i >= N * N + N {
        format!(
            "{}{}{}.",
            letter(i / N / N - 1),
            letter((i / N) % N + N - 1),
            letter(i)
        )
    } else if i >= N {
        format!("{}{}.", letter(i / N - 1), letter(i))
    } else {
        format!("{}.", letter(i))
    }
}

/// `I.`, `II.`, `III.`, `IV.`, …
#[must_use]
pub fn roman(_: &Items<'_>, i: usize) -> String {
    const TABLE: [(usize, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut n = i + 1;
    let mut out = String::new();
    for (value, numeral) in TABLE {
        while n >= value {
            n -= value;
            out.push_str(numeral);
        }
    }
    out.push('.');
    out
}

/// A list of items with enumerators (bullets, numbers, letters, …).
///
/// Enumerators are right-aligned, so `9.` and `10.` line up on the dot.
/// Items can be anything a [`Tree`] accepts as a child, including other
/// lists and trees.
#[derive(Clone, Debug)]
pub struct List {
    tree: Tree,
}

impl Default for List {
    fn default() -> Self {
        Self {
            tree: Tree::new()
                .enumerator(bullet)
                .indenter(|_, _| " ".to_string()),
        }
    }
}

impl List {
    /// Creates a bulleted list from items.
    #[must_use]
    pub fn new<I, N>(items: I) -> Self
    where
        I: IntoIterator<Item = N>,
        N: Into<Node>,
    {
        Self::default().items(items)
    }

    /// Adds an item. A [`List`] or root-less [`Tree`] becomes a sublist of
    /// the previous item.
    #[must_use]
    pub fn item(mut self, item: impl Into<Node>) -> Self {
        self.tree.push(item.into());
        self
    }

    /// Adds several items.
    #[must_use]
    pub fn items<I, N>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = N>,
        N: Into<Node>,
    {
        for i in items {
            self.tree.push(i.into());
        }
        self
    }

    /// Sets the enumerator: [`bullet`] (default), [`dash`], [`asterisk`],
    /// [`arabic`], [`alphabet`], [`roman`] or any closure.
    #[must_use]
    pub fn enumerator<F>(mut self, f: F) -> Self
    where
        F: Fn(&Items<'_>, usize) -> String + Send + Sync + 'static,
    {
        self.tree = self.tree.enumerator(f);
        self
    }

    /// Sets the indenter used in front of nested items and below multi-line
    /// items (default: a space).
    #[must_use]
    pub fn indenter<F>(mut self, f: F) -> Self
    where
        F: Fn(&Items<'_>, usize) -> String + Send + Sync + 'static,
    {
        self.tree = self.tree.indenter(f);
        self
    }

    /// Sets one style for every enumerator (default: `padding_right(1)`).
    #[must_use]
    pub fn enumerator_style(mut self, style: Style) -> Self {
        self.tree = self.tree.enumerator_style(style);
        self
    }

    /// Sets a per-item enumerator style.
    #[must_use]
    pub fn enumerator_style_func<F>(mut self, f: F) -> Self
    where
        F: Fn(&Items<'_>, usize) -> Style + Send + Sync + 'static,
    {
        self.tree = self.tree.enumerator_style_func(f);
        self
    }

    /// Sets one style for every indenter (default: `padding_right(1)`).
    #[must_use]
    pub fn indenter_style(mut self, style: Style) -> Self {
        self.tree = self.tree.indenter_style(style);
        self
    }

    /// Sets a per-item indenter style.
    #[must_use]
    pub fn indenter_style_func<F>(mut self, f: F) -> Self
    where
        F: Fn(&Items<'_>, usize) -> Style + Send + Sync + 'static,
    {
        self.tree = self.tree.indenter_style_func(f);
        self
    }

    /// Sets one style for every item.
    #[must_use]
    pub fn item_style(mut self, style: Style) -> Self {
        self.tree = self.tree.item_style(style);
        self
    }

    /// Sets a per-item style.
    #[must_use]
    pub fn item_style_func<F>(mut self, f: F) -> Self
    where
        F: Fn(&Items<'_>, usize) -> Style + Send + Sync + 'static,
    {
        self.tree = self.tree.item_style_func(f);
        self
    }

    /// Hides or shows the list.
    #[must_use]
    pub fn hide(mut self, hidden: bool) -> Self {
        self.tree = self.tree.hide(hidden);
        self
    }

    /// Only shows items `start..len - end`.
    #[must_use]
    pub fn offset(mut self, start: usize, end: usize) -> Self {
        self.tree = self.tree.offset(start, end);
        self
    }

    /// Style patched onto the selected item when rendered with a
    /// [`TreeState`].
    #[must_use]
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.tree = self.tree.highlight_style(style);
        self
    }

    /// The underlying tree.
    #[must_use]
    pub fn as_tree(&self) -> &Tree {
        &self.tree
    }

    /// Converts into the underlying tree.
    #[must_use]
    pub fn into_tree(self) -> Tree {
        self.tree
    }

    /// The items.
    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        self.tree.nodes()
    }

    /// Renders the list to a ratatui [`Text`].
    #[must_use]
    pub fn render(&self) -> Text<'static> {
        self.tree.render()
    }

    /// Renders the list to a string with ANSI escape codes.
    #[must_use]
    pub fn render_ansi(&self) -> String {
        self.tree.render_ansi()
    }
}

impl From<List> for Node {
    fn from(l: List) -> Self {
        Self::Tree(Box::new(l.tree))
    }
}

impl From<List> for Tree {
    fn from(l: List) -> Self {
        l.tree
    }
}

impl fmt::Display for List {
    /// Writes the list as plain text (no styling).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.tree, f)
    }
}

impl Widget for &List {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Widget::render(&self.tree, area, buf);
    }
}

impl StatefulWidget for &List {
    type State = TreeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TreeState) {
        StatefulWidget::render(&self.tree, area, buf, state);
    }
}
