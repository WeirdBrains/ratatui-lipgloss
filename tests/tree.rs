//! Tree and list tests.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::text::{Line, Span};
use ratatui::widgets::{StatefulWidget, Widget};
use ratatui_lipgloss::list::{self, List};
use ratatui_lipgloss::tree::{self, Leaf, Node, NodeRef, Tree, TreeState};
use ratatui_lipgloss::{Style, to_plain_string};

fn sample() -> Tree {
    Tree::new()
        .child("Foo")
        .child(
            tree::root("Bar")
                .child("Qux")
                .child(tree::root("Quux").child("Foo").child("Bar"))
                .child("Quuux"),
        )
        .child("Baz")
}

#[test]
fn default_and_rounded_enumerators() {
    assert_eq!(
        sample().to_string(),
        "├── Foo\n├── Bar\n│   ├── Qux\n│   ├── Quux\n│   │   ├── Foo\n│   │   └── Bar\n│   └── Quuux\n└── Baz"
    );
    let rounded = sample().enumerator(tree::rounded_enumerator).to_string();
    assert!(rounded.ends_with("╰── Baz"));
    assert!(rounded.contains("│   │   ╰── Bar"));
}

#[test]
fn root_hidden_offset_and_collapsed() {
    let t = tree::root(".").children(["a", "b", "c", "d"]);
    assert_eq!(t.to_string(), ".\n├── a\n├── b\n├── c\n└── d");
    assert_eq!(t.clone().offset(1, 1).to_string(), ".\n├── b\n└── c");
    let hidden = tree::root(".")
        .child("a")
        .child(Leaf::new("b").hide(true))
        .child("c");
    assert_eq!(hidden.to_string(), ".\n├── a\n└── c");
    assert!(Tree::new().child("x").hide(true).to_string().is_empty());
    let collapsed = tree::root(".").child(tree::root("dir").child("file").collapsed(true));
    assert_eq!(collapsed.to_string(), ".\n└── dir");
    assert!(tree::root("x").collapsed(true).is_collapsed());
}

#[test]
fn rootless_subtree_attaches_to_previous_node() {
    let t = Tree::new()
        .child("parent")
        .child(Tree::new().child("kid"))
        .child(Tree::new().child("second kid"));
    assert_eq!(t.to_string(), "└── parent\n    ├── kid\n    └── second kid");
    // a root-less tree as the first child is kept as is
    let t = Tree::new().child(Tree::new().child("x"));
    assert_eq!(t.to_string(), "└── \n    └── x");
}

#[test]
fn styles_apply_to_parts() {
    let t = tree::root("root")
        .child("a")
        .child("b")
        .root_style(Style::new().foreground(Color::Magenta))
        .item_style_func(|_, i| {
            if i == 0 {
                Style::new().foreground(Color::Red)
            } else {
                Style::new().italic(true)
            }
        })
        .enumerator_style(Style::new().foreground(Color::Blue).padding_right(1));
    let mut buf = Buffer::empty(Rect::new(0, 0, 8, 3));
    Widget::render(&t, buf.area, &mut buf);
    assert_eq!(buf[(0, 0)].fg, Color::Magenta);
    assert_eq!(buf[(0, 1)].fg, Color::Blue);
    assert_eq!(buf[(4, 1)].fg, Color::Red);
    assert!(buf[(4, 2)].modifier.contains(Modifier::ITALIC));
    assert!(t.render_ansi().contains("\x1b["));
}

#[test]
fn custom_enumerator_indenter_and_width() {
    let t = Tree::new()
        .child("a")
        .child(tree::root("b").child("c"))
        .enumerator(|_, _| "->".into())
        .indenter(|_, _| "->".into())
        .indenter_style(Style::new().padding_right(1))
        .width(10);
    let s = t.render();
    assert_eq!(to_plain_string(&s), "-> a      \n-> b      \n-> -> c   ");
    assert!(s.lines.iter().all(|l| l.width() == 10));
}

#[test]
fn multiline_items_continue_with_indenter() {
    let t = Tree::new().child("one\ntwo").child("three");
    assert_eq!(t.to_string(), "├── one\n│   two\n└── three");
}

#[test]
fn mixed_width_enumerators_are_right_aligned() {
    let t = Tree::new()
        .children(["a", "b"])
        .enumerator(|_, i| if i == 0 { "-".into() } else { "--->".into() });
    assert_eq!(t.to_string(), "   - a\n---> b");
}

#[test]
fn node_access_and_conversions() {
    let t = Tree::new()
        .child(Span::raw("span"))
        .child(Line::from("line"))
        .child(String::from("string"))
        .child({
            let owned = String::from("ref");
            Node::from(&owned)
        })
        .child(tree::root("sub").child("leaf"));
    assert_eq!(t.nodes().len(), 5);
    assert_eq!(t.nodes()[0].value_string(), "span");
    assert!(!t.nodes()[0].is_hidden());
    assert!(t.nodes()[0].children().is_empty());
    assert_eq!(t.nodes()[4].children().len(), 1);
    match t.node_at(&[4, 0]) {
        Some(NodeRef::Leaf(l)) => assert_eq!(to_plain_string(l.value()), "leaf"),
        other => panic!("unexpected {other:?}"),
    }
    assert!(matches!(t.node_at(&[]), Some(NodeRef::Tree(_))));
    assert!(t.node_at(&[9]).is_none());
    assert_eq!(t.node_at(&[4]).unwrap().children().len(), 1);
    assert_eq!(to_plain_string(t.node_at(&[4]).unwrap().value()), "sub");
    assert!(matches!(Node::from(List::new(["x"])), Node::Tree(_)));
    let tree_from_list: Tree = List::new(["x"]).into();
    assert_eq!(tree_from_list.to_string(), "• x");
    assert!(!tree_from_list.is_hidden());
    assert!(Leaf::new("x").value().width() == 1 && !Leaf::new("x").is_hidden());
}

#[test]
fn children_view_helpers() {
    let t = Tree::new().children(["a", "b", "c"]).enumerator(|c, i| {
        assert!(!c.is_empty());
        assert_eq!(c.iter().count(), c.len());
        format!("{}/{}", c.get(i).unwrap().value_string(), c.len())
    });
    assert_eq!(t.to_string(), "a/3 a\nb/3 b\nc/3 c");
}

#[test]
fn stateful_navigation_collapse_and_mouse() {
    let t = tree::root("~")
        .child(tree::root("src").child("lib.rs").child("main.rs"))
        .child("README.md")
        .highlight_style(Style::new().background(Color::Blue));
    let mut state = TreeState::new();
    assert_eq!(state.selected(), None);
    state.select_previous(&t);
    assert_eq!(state.selected(), Some(&[1][..]));
    state.select_first(&t);
    assert_eq!(state.selected(), Some(&[][..]));
    state.select_next(&t);
    state.select_next(&t);
    assert_eq!(state.selected(), Some(&[0, 0][..]));
    state.select_last(&t);
    assert_eq!(state.selected(), Some(&[1][..]));
    state.select_next(&t); // stays on the last node
    assert_eq!(state.selected(), Some(&[1][..]));

    // collapse "src"
    assert!(state.toggle(vec![0]));
    assert!(state.is_collapsed(&[0]));
    assert_eq!(
        t.visible_paths(Some(&state)),
        vec![vec![], vec![0], vec![1]]
    );
    state.expand(&[0]);
    assert!(!state.is_collapsed(&[0]));
    state.collapse(vec![0]);
    assert!(!state.toggle(vec![0]));
    state.select(Some(vec![0]));
    state.toggle_selected();
    assert!(state.is_collapsed(&[0]));
    state.toggle_selected();

    let mut buf = Buffer::empty(Rect::new(0, 0, 14, 3));
    StatefulWidget::render(&t, buf.area, &mut buf, &mut state);
    // three lines visible: "~", "├── src", "│   ├── lib.rs"
    assert_eq!(buf[(4, 1)].symbol(), "s");
    assert_eq!(buf[(4, 1)].bg, Color::Blue);
    assert_eq!(buf[(13, 1)].bg, Color::Blue); // full-width highlight
    assert_eq!(buf[(4, 2)].bg, Color::Reset);
    assert_eq!(state.path_at(5, 2), Some(vec![0, 0]));
    assert_eq!(state.path_at(5, 9), None);

    // selecting the last node scrolls it into view
    state.select_last(&t);
    let mut buf = Buffer::empty(Rect::new(0, 0, 14, 3));
    StatefulWidget::render(&t, buf.area, &mut buf, &mut state);
    assert_eq!(state.offset(), 2);
    assert_eq!(state.path_at(0, 2), Some(vec![1]));
    // and back up again
    state.select_first(&t);
    StatefulWidget::render(&t, buf.area, &mut buf, &mut state);
    assert_eq!(state.offset(), 0);
    state.set_offset(1);
    assert_eq!(state.offset(), 1);

    let mut empty = TreeState::default();
    empty.select_next(&Tree::new());
    assert_eq!(empty.selected(), None);
}

#[test]
fn list_enumerators() {
    let items = || List::new(["a", "b", "c"]);
    assert_eq!(items().to_string(), "• a\n• b\n• c");
    assert_eq!(items().enumerator(list::dash).to_string(), "- a\n- b\n- c");
    assert_eq!(
        items().enumerator(list::asterisk).to_string(),
        "* a\n* b\n* c"
    );
    assert_eq!(
        items().enumerator(list::arabic).to_string(),
        "1. a\n2. b\n3. c"
    );
    assert_eq!(
        items().enumerator(list::alphabet).to_string(),
        "A. a\nB. b\nC. c"
    );
    assert_eq!(
        items().enumerator(list::roman).to_string(),
        "  I. a\n II. b\nIII. c"
    );
    assert_eq!(List::new(["x"]).as_tree().nodes().len(), 1);
}

#[test]
fn enumerator_values() {
    let l = List::new(["x"]).enumerator(|items, i| {
        format!(
            "{} {} {}",
            list::alphabet(items, i + 26),
            list::alphabet(items, 26 * 26 + 26),
            list::roman(items, 1993)
        )
    });
    assert_eq!(l.to_string(), "AA. AAA. MCMXCIV. x");
    let l = List::new(["x"]).enumerator(|items, _| list::alphabet(items, 27));
    assert_eq!(l.to_string(), "AB. x");
}

#[test]
fn nested_lists_and_styles() {
    let l = List::new(["Foo", "Bar"])
        .item(List::new(["Hi", "Hello"]).enumerator(list::arabic))
        .item("Qux")
        .item(Tree::new().child("tree").child("in list"))
        .item(
            tree::root("Own")
                .child("own")
                .child("renderer")
                .enumerator(tree::default_enumerator),
        );
    assert_eq!(
        l.to_string(),
        "• Foo\n• Bar\n  1. Hi\n  2. Hello\n• Qux\n  • tree\n  • in list\n• Own\n  ├── own\n  └── renderer"
    );
    let styled = List::new(["a", "b"])
        .enumerator_style_func(|_, i| {
            Style::new()
                .foreground(if i == 0 { Color::Red } else { Color::Green })
                .padding_right(1)
        })
        .item_style(Style::new().bold(true))
        .indenter_style(Style::new().padding_right(1))
        .indenter_style_func(|_, _| Style::new().padding_right(1))
        .item_style_func(|_, _| Style::new().underline(true))
        .indenter(|_, _| " ".into())
        .highlight_style(Style::new().reverse(true));
    let mut buf = Buffer::empty(Rect::new(0, 0, 3, 2));
    Widget::render(&styled, buf.area, &mut buf);
    assert_eq!(buf[(0, 0)].fg, Color::Red);
    assert_eq!(buf[(0, 1)].fg, Color::Green);
    assert!(buf[(2, 0)].modifier.contains(Modifier::UNDERLINED));
    assert!(styled.render_ansi().contains("\x1b["));
    assert_eq!(styled.nodes().len(), 2);
    assert_eq!(
        List::new(["a", "b", "c"]).offset(1, 0).to_string(),
        "• b\n• c"
    );
    assert!(List::new(["a"]).hide(true).to_string().is_empty());
    assert_eq!(List::default().items(["z"]).into_tree().to_string(), "• z");

    let mut state = TreeState::new();
    state.select_next(styled.as_tree());
    let mut buf = Buffer::empty(Rect::new(0, 0, 3, 2));
    StatefulWidget::render(&styled, buf.area, &mut buf, &mut state);
    assert!(buf[(2, 0)].modifier.contains(Modifier::REVERSED));
}

#[test]
fn tree_in_list_in_tree() {
    let t = tree::root("menu")
        .child("drinks")
        .child(List::new(["tea", "coffee"]).enumerator(list::dash))
        .child("food");
    assert_eq!(
        t.to_string(),
        "menu\n├── drinks\n│   - tea\n│   - coffee\n└── food"
    );
    assert!(format!("{t:?}").contains("Tree"));
}
