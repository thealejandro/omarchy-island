//! Which screen the island lives on.

use crate::shape::Rect;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Monitor {
    /// In virtual-screen pixels: secondary screens can have negative x or y.
    pub rect: Rect,
    pub primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MonitorChoice {
    #[default]
    Primary,
    /// 1-based, counting screens left to right (then top to bottom), the way
    /// they sit in Windows' display settings.
    Number(usize),
}

impl MonitorChoice {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "primary" => Some(Self::Primary),
            _ => s.parse().ok().filter(|n| *n >= 1).map(Self::Number),
        }
    }
}

/// Index into `monitors` of the screen to use: the chosen one while it is
/// connected, else the primary, else the first. `None` with no screens.
pub fn pick(monitors: &[Monitor], choice: MonitorChoice) -> Option<usize> {
    let primary = monitors.iter().position(|m| m.primary);
    let chosen = match choice {
        MonitorChoice::Primary => None,
        MonitorChoice::Number(n) => {
            let mut order: Vec<usize> = (0..monitors.len()).collect();
            order.sort_by(|&a, &b| {
                let (ra, rb) = (monitors[a].rect, monitors[b].rect);
                ra.x.total_cmp(&rb.x).then(ra.y.total_cmp(&rb.y))
            });
            order.get(n - 1).copied()
        }
    };
    chosen
        .or(primary)
        .or(if monitors.is_empty() { None } else { Some(0) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon(x: f32, y: f32, primary: bool) -> Monitor {
        Monitor {
            rect: Rect {
                x,
                y,
                width: 2560.0,
                height: 1440.0,
            },
            primary,
        }
    }

    // As Windows lists them: the primary first, a screen on its left second.
    fn two() -> Vec<Monitor> {
        vec![mon(0.0, 0.0, true), mon(-2560.0, 0.0, false)]
    }

    #[test]
    fn primary_by_default() {
        assert_eq!(pick(&two(), MonitorChoice::Primary), Some(0));
    }

    #[test]
    fn numbers_count_left_to_right_then_top_to_bottom() {
        assert_eq!(pick(&two(), MonitorChoice::Number(1)), Some(1));
        assert_eq!(pick(&two(), MonitorChoice::Number(2)), Some(0));
        let stacked = vec![mon(0.0, 1440.0, false), mon(0.0, 0.0, true)];
        assert_eq!(pick(&stacked, MonitorChoice::Number(1)), Some(1));
    }

    #[test]
    fn an_unplugged_screen_falls_back_to_the_primary() {
        assert_eq!(pick(&two(), MonitorChoice::Number(3)), Some(0));
        assert_eq!(
            pick(&[mon(0.0, 0.0, false)], MonitorChoice::Number(2)),
            Some(0)
        );
        assert_eq!(pick(&[], MonitorChoice::Primary), None);
    }

    #[test]
    fn choice_names() {
        assert_eq!(
            MonitorChoice::parse("primary"),
            Some(MonitorChoice::Primary)
        );
        assert_eq!(MonitorChoice::parse("2"), Some(MonitorChoice::Number(2)));
        for bad in ["0", "-1", "dos", ""] {
            assert_eq!(MonitorChoice::parse(bad), None);
        }
    }
}
