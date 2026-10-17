//! Positions (alignment) and CSS-like box sides.

use ratatui::layout::{Alignment, HorizontalAlignment, VerticalAlignment};
use ratatui::widgets::Padding;

/// A position along an axis, from `0.0` (top / left) to `1.0`
/// (bottom / right).
///
/// Like Lip Gloss, any fraction is allowed: `Position::new(0.2)` aligns a
/// block 20% of the way along the axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Position(f32);

impl Position {
    /// Top edge (`0.0`).
    pub const TOP: Self = Self(0.0);
    /// Bottom edge (`1.0`).
    pub const BOTTOM: Self = Self(1.0);
    /// Left edge (`0.0`).
    pub const LEFT: Self = Self(0.0);
    /// Right edge (`1.0`).
    pub const RIGHT: Self = Self(1.0);
    /// Center (`0.5`).
    pub const CENTER: Self = Self(0.5);

    /// Creates a position, clamping to `0.0..=1.0` (NaN becomes `0.0`).
    #[must_use]
    pub fn new(value: f32) -> Self {
        if value.is_nan() {
            Self(0.0)
        } else {
            Self(value.clamp(0.0, 1.0))
        }
    }

    /// The position as a fraction in `0.0..=1.0`.
    #[must_use]
    pub const fn value(self) -> f32 {
        self.0
    }

    /// Splits `gap` cells into `(before, after)` according to this position.
    #[must_use]
    pub fn split(self, gap: usize) -> (usize, usize) {
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let before = ((gap as f32 * self.0).floor() as usize).min(gap);
        (before, gap - before)
    }
}

impl From<f32> for Position {
    fn from(v: f32) -> Self {
        Self::new(v)
    }
}

impl From<HorizontalAlignment> for Position {
    fn from(a: HorizontalAlignment) -> Self {
        match a {
            Alignment::Left => Self::LEFT,
            Alignment::Center => Self::CENTER,
            Alignment::Right => Self::RIGHT,
        }
    }
}

impl From<VerticalAlignment> for Position {
    fn from(a: VerticalAlignment) -> Self {
        match a {
            VerticalAlignment::Top => Self::TOP,
            VerticalAlignment::Center => Self::CENTER,
            VerticalAlignment::Bottom => Self::BOTTOM,
        }
    }
}

/// Values for the four sides of a box, in CSS order (top, right, bottom,
/// left).
///
/// Converts from CSS-style shorthands, so `padding(2)`, `padding((1, 4))`,
/// `padding((1, 4, 2))` and `padding((2, 4, 3, 1))` all work exactly like
/// Lip Gloss's `Padding(...)` / `Margin(...)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sides<T = u16> {
    /// Top side.
    pub top: T,
    /// Right side.
    pub right: T,
    /// Bottom side.
    pub bottom: T,
    /// Left side.
    pub left: T,
}

impl<T: Copy> Sides<T> {
    /// Creates sides in CSS (clockwise from top) order.
    #[must_use]
    pub const fn new(top: T, right: T, bottom: T, left: T) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// The same value on all sides.
    #[must_use]
    pub const fn all(v: T) -> Self {
        Self::new(v, v, v, v)
    }
}

impl<T: Copy> From<T> for Sides<T> {
    fn from(v: T) -> Self {
        Self::all(v)
    }
}

impl<T: Copy> From<(T, T)> for Sides<T> {
    /// `(vertical, horizontal)`
    fn from((v, h): (T, T)) -> Self {
        Self::new(v, h, v, h)
    }
}

impl<T: Copy> From<(T, T, T)> for Sides<T> {
    /// `(top, horizontal, bottom)`
    fn from((t, h, b): (T, T, T)) -> Self {
        Self::new(t, h, b, h)
    }
}

impl<T: Copy> From<(T, T, T, T)> for Sides<T> {
    /// `(top, right, bottom, left)`
    fn from((t, r, b, l): (T, T, T, T)) -> Self {
        Self::new(t, r, b, l)
    }
}

impl From<Padding> for Sides {
    fn from(p: Padding) -> Self {
        Self::new(p.top, p.right, p.bottom, p.left)
    }
}

impl From<Sides> for Padding {
    fn from(s: Sides) -> Self {
        Self::new(s.left, s.right, s.top, s.bottom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_shorthand() {
        assert_eq!(Sides::from(2u16), Sides::new(2, 2, 2, 2));
        assert_eq!(Sides::from((2u16, 4)), Sides::new(2, 4, 2, 4));
        assert_eq!(Sides::from((1u16, 4, 2)), Sides::new(1, 4, 2, 4));
        assert_eq!(Sides::from((2u16, 4, 3, 1)), Sides::new(2, 4, 3, 1));
        assert_eq!(
            Sides::from((true, false)),
            Sides::new(true, false, true, false)
        );
    }

    #[test]
    fn position_split() {
        assert_eq!(Position::CENTER.split(3), (1, 2));
        assert_eq!(Position::RIGHT.split(3), (3, 0));
        assert_eq!(Position::LEFT.split(3), (0, 3));
        assert_eq!(Position::new(7.0), Position::BOTTOM);
        assert_eq!(Position::from(Alignment::Center), Position::CENTER);
    }

    #[test]
    fn padding_roundtrip() {
        let s = Sides::new(1, 2, 3, 4);
        assert_eq!(Sides::from(Padding::from(s)), s);
    }
}
