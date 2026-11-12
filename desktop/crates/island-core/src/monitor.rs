//! Which screens the island lives on.

use crate::shape::Rect;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Monitor {
    /// In virtual-screen pixels: secondary screens can have negative x or y.
    pub rect: Rect,
    pub primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MonitorChoice {
    #[default]
    Primary,
    /// 1-based, counting screens left to right (then top to bottom), the way
    /// they sit in Windows' display settings.
    Number(usize),
    /// Every connected screen: an island on each.
    All,
    /// These screens (1-based, as `Number`), each with an island.
    List(Vec<usize>),
}

impl MonitorChoice {
    /// `primary`, `all`, a number, or a comma-separated list (`1,3`).
    /// Anything else: `None`.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        match s {
            "primary" => Some(Self::Primary),
            "all" => Some(Self::All),
            _ if s.contains(',') => {
                let ns: Option<Vec<usize>> = s
                    .split(',')
                    .map(|p| p.trim().parse().ok().filter(|n| *n >= 1))
                    .collect();
                ns.map(Self::List)
            }
            _ => s.parse().ok().filter(|n| *n >= 1).map(Self::Number),
        }
    }
}

/// Indices into `monitors` of the screens to use: the chosen ones while they
/// are connected, deduplicated, in the order they sit left to right (then
/// top to bottom); the primary when none of them is there, else the first.
/// Empty only with no screens.
pub fn pick_all(monitors: &[Monitor], choice: &MonitorChoice) -> Vec<usize> {
    // The screens as they sit: left to right, then top to bottom.
    let mut order: Vec<usize> = (0..monitors.len()).collect();
    order.sort_by(|&a, &b| {
        let (ra, rb) = (monitors[a].rect, monitors[b].rect);
        ra.x.total_cmp(&rb.x).then(ra.y.total_cmp(&rb.y))
    });
    let chosen: Vec<usize> = match choice {
        MonitorChoice::Primary => Vec::new(),
        MonitorChoice::Number(n) => n
            .checked_sub(1)
            .and_then(|i| order.get(i).copied())
            .into_iter()
            .collect(),
        MonitorChoice::All => order.clone(),
        MonitorChoice::List(ns) => ns
            .iter()
            .filter_map(|n| n.checked_sub(1).and_then(|i| order.get(i).copied()))
            .collect(),
    };
    // Connected, deduplicated, left to right.
    let picked: Vec<usize> = order.into_iter().filter(|i| chosen.contains(i)).collect();
    if !picked.is_empty() {
        return picked;
    }
    monitors
        .iter()
        .position(|m| m.primary)
        .or(if monitors.is_empty() { None } else { Some(0) })
        .into_iter()
        .collect()
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

    fn three() -> Vec<Monitor> {
        vec![
            mon(0.0, 0.0, true),
            mon(-2560.0, 0.0, false),
            mon(2560.0, 0.0, false),
        ]
    }

    #[test]
    fn primary_by_default() {
        assert_eq!(pick_all(&two(), &MonitorChoice::Primary), vec![0]);
    }

    #[test]
    fn numbers_count_left_to_right_then_top_to_bottom() {
        assert_eq!(pick_all(&two(), &MonitorChoice::Number(1)), vec![1]);
        assert_eq!(pick_all(&two(), &MonitorChoice::Number(2)), vec![0]);
        let stacked = vec![mon(0.0, 1440.0, false), mon(0.0, 0.0, true)];
        assert_eq!(pick_all(&stacked, &MonitorChoice::Number(1)), vec![1]);
    }

    #[test]
    fn an_unplugged_screen_falls_back_to_the_primary() {
        assert_eq!(pick_all(&two(), &MonitorChoice::Number(3)), vec![0]);
        assert_eq!(
            pick_all(&[mon(0.0, 0.0, false)], &MonitorChoice::Number(2)),
            vec![0]
        );
        assert_eq!(pick_all(&[], &MonitorChoice::Primary), Vec::<usize>::new());
    }

    #[test]
    fn all_takes_every_screen_left_to_right() {
        assert_eq!(pick_all(&three(), &MonitorChoice::All), vec![1, 0, 2]);
        assert_eq!(pick_all(&two(), &MonitorChoice::All), vec![1, 0]);
        assert_eq!(pick_all(&[], &MonitorChoice::All), Vec::<usize>::new());
    }

    #[test]
    fn a_list_takes_its_connected_screens() {
        // The left screen (1) and one that isn't there (3): only the first.
        assert_eq!(pick_all(&two(), &MonitorChoice::List(vec![1, 3])), vec![1]);
        // Repeated and out of order: deduplicated, left to right.
        assert_eq!(
            pick_all(&three(), &MonitorChoice::List(vec![3, 1, 1])),
            vec![1, 2]
        );
        // An empty list is nothing chosen: the primary.
        assert_eq!(pick_all(&two(), &MonitorChoice::List(vec![])), vec![0]);
        assert_eq!(pick_all(&two(), &MonitorChoice::List(vec![9])), vec![0]);
    }

    #[test]
    fn choice_names() {
        assert_eq!(
            MonitorChoice::parse("primary"),
            Some(MonitorChoice::Primary)
        );
        assert_eq!(MonitorChoice::parse("all"), Some(MonitorChoice::All));
        assert_eq!(MonitorChoice::parse("2"), Some(MonitorChoice::Number(2)));
        assert_eq!(MonitorChoice::parse(" 2 "), Some(MonitorChoice::Number(2)));
        assert_eq!(MonitorChoice::parse(" all"), Some(MonitorChoice::All));
        assert_eq!(
            MonitorChoice::parse("1,3"),
            Some(MonitorChoice::List(vec![1, 3]))
        );
        assert_eq!(
            MonitorChoice::parse("2, 1"),
            Some(MonitorChoice::List(vec![2, 1]))
        );
        for bad in ["0", "-1", "dos", "", "1,", ",2", "1,0", "1,dos"] {
            assert_eq!(MonitorChoice::parse(bad), None, "{bad}");
        }
    }
}
