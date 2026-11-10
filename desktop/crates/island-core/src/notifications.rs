//! Notifications as they arrive: the system's list read again when a new
//! one shows, and only those not seen before told. The list read at start
//! is what was there already: no news.

/// Segoe Fluent Icons (and MDL2): the notification bell.
pub const ICON: &str = "\u{EA8F}";

/// A notification, as far as a glance tells it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub id: u32,
    /// The app that sent it.
    pub app: String,
    /// Its title (its first line of text).
    pub title: String,
}

impl Toast {
    /// One line of text, whatever the app sent (line breaks, runs of spaces).
    pub fn line(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

/// The notifications seen, by id.
#[derive(Debug, Default)]
pub struct Seen {
    ids: Vec<u32>,
    primed: bool,
}

impl Seen {
    /// The list now; the ones in it not seen before, oldest first (none the
    /// first time). The ones gone are forgotten.
    pub fn new_ones(&mut self, now: Vec<Toast>) -> Vec<Toast> {
        let ids: Vec<u32> = now.iter().map(|t| t.id).collect();
        let fresh = if self.primed {
            now.into_iter()
                .filter(|t| !self.ids.contains(&t.id))
                .collect()
        } else {
            Vec::new()
        };
        self.ids = ids;
        self.primed = true;
        fresh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u32) -> Toast {
        Toast {
            id,
            app: "Mail".into(),
            title: format!("n{id}"),
        }
    }

    #[test]
    fn only_new_ones_are_told() {
        let mut s = Seen::default();
        assert_eq!(s.new_ones(vec![t(1), t(2)]), vec![]);
        assert_eq!(s.new_ones(vec![t(1), t(2), t(3)]), vec![t(3)]);
        // Read again with nothing new (another window showed): nothing.
        assert_eq!(s.new_ones(vec![t(1), t(2), t(3)]), vec![]);
        // One dismissed, two new.
        assert_eq!(s.new_ones(vec![t(2), t(3), t(4), t(5)]), vec![t(4), t(5)]);
        // A dismissed one is forgotten; its id coming back is new.
        assert_eq!(s.new_ones(vec![t(1)]), vec![t(1)]);
    }

    #[test]
    fn text_is_one_line() {
        assert_eq!(Toast::line("  Hola\n  mundo\t !"), "Hola mundo !");
        assert_eq!(Toast::line("\n"), "");
    }
}
