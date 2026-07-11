//! Reusable vertical scroll state for line-based lists.
//!
//! Deliberately content-agnostic: it tracks a line offset and, given the focused
//! line range and the viewport height, scrolls the minimum needed to keep the
//! focus visible. Used by the Global left nav today; reusable by any scrolling
//! list (Catalog, Projects) as those grow.

#[derive(Debug, Default, Clone, Copy)]
pub struct Scroll {
    pub offset: usize,
}

impl Scroll {
    /// Move the offset the minimum needed so lines `[start, start + len)` are
    /// visible in a `height`-line viewport over a `total`-line list.
    pub fn focus(&mut self, start: usize, len: usize, height: usize, total: usize) {
        if height == 0 {
            self.offset = 0;
            return;
        }
        if start < self.offset {
            self.offset = start;
        }
        let end = start + len;
        if end > self.offset + height {
            self.offset = end - height;
        }
        self.offset = self.offset.min(total.saturating_sub(height));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolls_down_to_reveal_focus_below() {
        let mut s = Scroll::default();
        s.focus(10, 2, 5, 20); // lines 10,11 into a 5-line window
        assert_eq!(s.offset, 7); // shows 7..12
    }

    #[test]
    fn scrolls_up_to_reveal_focus_above() {
        let mut s = Scroll { offset: 7 };
        s.focus(2, 2, 5, 20);
        assert_eq!(s.offset, 2);
    }

    #[test]
    fn clamps_to_max_offset() {
        let mut s = Scroll::default();
        s.focus(19, 1, 5, 20);
        assert_eq!(s.offset, 15); // last window 15..20
    }

    #[test]
    fn no_scroll_when_focus_already_visible() {
        let mut s = Scroll { offset: 3 };
        s.focus(4, 2, 5, 20); // 4,5 already within 3..8
        assert_eq!(s.offset, 3);
    }

    #[test]
    fn zero_height_resets() {
        let mut s = Scroll { offset: 9 };
        s.focus(0, 1, 0, 20);
        assert_eq!(s.offset, 0);
    }
}
