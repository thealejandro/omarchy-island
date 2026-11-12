//! Paired Bluetooth devices coming and going: what the system's device
//! watcher tells, turned into glances. The watcher lists every paired
//! device first; only changes after that list is done are told, and only
//! once they hold: Low Energy devices (phones, bands) often connect for a
//! moment on their own, and a blip like that is no news.

/// Segoe Fluent Icons (and MDL2).
pub const ICON: crate::icons::Icon = crate::icons::Icon::Bluetooth;
/// The icon's colour (linear RGB), Bluetooth's own blue.
pub const TINT: [f32; 3] = [0.2, 0.5, 1.0];
/// How long a change must hold before it's told, in ms.
pub const SETTLE_MS: u64 = 5000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Connected(String),
    Disconnected(String),
}

#[derive(Debug, Default)]
struct Device {
    id: String,
    name: String,
    /// Connected, as last told (or as first listed).
    told: bool,
    /// Connected now, and since when.
    now: bool,
    since: u64,
}

/// The paired devices as last seen, and whether the first list is done.
/// Times are ms on any steady clock.
#[derive(Debug, Default)]
pub struct Links {
    devices: Vec<Device>,
    ready: bool,
}

impl Links {
    /// The first list is done: changes from now on are told.
    pub fn ready(&mut self) {
        self.ready = true;
    }

    /// A device paired (or listed at first).
    pub fn added(&mut self, id: &str, name: &str, connected: bool, at: u64) {
        self.devices.retain(|d| d.id != id);
        self.devices.push(Device {
            id: id.into(),
            name: name.into(),
            // A new pairing that comes connected is news; the first list
            // is not.
            told: if self.ready { false } else { connected },
            now: connected,
            since: at,
        });
    }

    /// Something of a device changed; `None` for what the update leaves out.
    pub fn updated(&mut self, id: &str, name: Option<&str>, connected: Option<bool>, at: u64) {
        let ready = self.ready;
        let Some(d) = self.devices.iter_mut().find(|d| d.id == id) else {
            return;
        };
        if let Some(n) = name.filter(|n| !n.is_empty()) {
            d.name = n.into();
        }
        let Some(now) = connected else { return };
        if now != d.now {
            d.now = now;
            d.since = at;
        }
        if !ready {
            d.told = now;
        }
    }

    /// Unpaired: gone without a word.
    pub fn removed(&mut self, id: &str) {
        self.devices.retain(|d| d.id != id);
    }

    /// The changes that have held, told once each; and in how long the
    /// next one still waiting will have, if one is.
    pub fn due(&mut self, at: u64) -> (Vec<Event>, Option<u64>) {
        let mut told = Vec::new();
        let mut next: Option<u64> = None;
        for d in self.devices.iter_mut().filter(|d| d.now != d.told) {
            let when = d.since + SETTLE_MS;
            if at >= when {
                d.told = d.now;
                told.push(if d.now {
                    Event::Connected(d.name.clone())
                } else {
                    Event::Disconnected(d.name.clone())
                });
            } else {
                next = Some(next.map_or(when - at, |n| n.min(when - at)));
            }
        }
        (told, next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: u64 = SETTLE_MS;

    #[test]
    fn the_first_list_is_quiet() {
        let mut l = Links::default();
        l.added("a", "Buds", true, 0);
        l.updated("a", None, Some(false), 10);
        l.ready();
        assert_eq!(l.due(S + 100), (vec![], None));
        l.updated("a", None, Some(true), S + 200);
        assert_eq!(l.due(2 * S + 200).0, vec![Event::Connected("Buds".into())]);
    }

    #[test]
    fn a_change_is_told_once_it_holds() {
        let mut l = Links::default();
        l.added("a", "Buds", false, 0);
        l.ready();
        l.updated("a", Some("Buds Pro"), Some(true), 1000);
        assert_eq!(l.due(1000), (vec![], Some(S)));
        assert_eq!(l.due(1000 + S - 1), (vec![], Some(1)));
        assert_eq!(
            l.due(1000 + S),
            (vec![Event::Connected("Buds Pro".into())], None)
        );
        // Told once.
        assert_eq!(l.due(1000 + 2 * S), (vec![], None));
        l.updated("a", Some(""), Some(false), 9 * S);
        assert_eq!(
            l.due(10 * S).0,
            vec![Event::Disconnected("Buds Pro".into())]
        );
    }

    #[test]
    fn a_blip_is_no_news() {
        let mut l = Links::default();
        l.added("p", "4", false, 0);
        l.added("k", "Keys", true, 0);
        l.ready();
        l.updated("p", None, Some(true), 100);
        l.updated("p", None, Some(false), 2000);
        // The same for a connected one dropping for a moment.
        l.updated("k", None, Some(false), 2500);
        assert_eq!(l.due(2600), (vec![], Some(S - 100)));
        l.updated("k", None, Some(true), 3000);
        assert_eq!(l.due(S + 3000), (vec![], None));
    }

    #[test]
    fn a_new_pairing_tells_if_connected() {
        let mut l = Links::default();
        l.ready();
        l.added("b", "Mouse", true, 0);
        l.added("c", "Keys", false, 0);
        assert_eq!(l.due(S).0, vec![Event::Connected("Mouse".into())]);
        l.removed("b");
        l.updated("b", None, Some(false), S);
        assert_eq!(l.due(3 * S), (vec![], None));
    }
}
