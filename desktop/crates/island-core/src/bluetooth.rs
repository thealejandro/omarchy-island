//! Paired Bluetooth devices coming and going: what the system's device
//! watcher tells, turned into glances. The watcher lists every paired
//! device first; only changes after that list is done are told.

/// Segoe Fluent Icons (and MDL2).
pub const ICON: &str = "\u{E702}";
/// The icon's colour (linear RGB), Bluetooth's own blue.
pub const TINT: [f32; 3] = [0.2, 0.5, 1.0];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Connected(String),
    Disconnected(String),
}

#[derive(Debug, Default)]
struct Device {
    id: String,
    name: String,
    connected: bool,
}

/// The paired devices as last told, and whether the first list is done.
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
    pub fn added(&mut self, id: &str, name: &str, connected: bool) -> Option<Event> {
        self.devices.retain(|d| d.id != id);
        self.devices.push(Device {
            id: id.into(),
            name: name.into(),
            connected,
        });
        (self.ready && connected).then(|| Event::Connected(name.into()))
    }

    /// Something of a device changed; `None` for what the update leaves out.
    pub fn updated(
        &mut self,
        id: &str,
        name: Option<&str>,
        connected: Option<bool>,
    ) -> Option<Event> {
        let d = self.devices.iter_mut().find(|d| d.id == id)?;
        if let Some(n) = name.filter(|n| !n.is_empty()) {
            d.name = n.into();
        }
        let now = connected?;
        if now == d.connected {
            return None;
        }
        d.connected = now;
        if !self.ready {
            return None;
        }
        let name = d.name.clone();
        Some(if now {
            Event::Connected(name)
        } else {
            Event::Disconnected(name)
        })
    }

    /// Unpaired: gone without a word.
    pub fn removed(&mut self, id: &str) {
        self.devices.retain(|d| d.id != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_list_is_quiet() {
        let mut l = Links::default();
        assert_eq!(l.added("a", "Buds", true), None);
        assert_eq!(l.updated("a", None, Some(false)), None);
        l.ready();
        assert_eq!(
            l.updated("a", None, Some(true)),
            Some(Event::Connected("Buds".into()))
        );
    }

    #[test]
    fn only_a_change_of_connection_is_told() {
        let mut l = Links::default();
        l.added("a", "Buds", false);
        l.ready();
        assert_eq!(l.updated("a", Some("Buds Pro"), None), None);
        assert_eq!(l.updated("a", None, Some(false)), None);
        assert_eq!(
            l.updated("a", None, Some(true)),
            Some(Event::Connected("Buds Pro".into()))
        );
        assert_eq!(
            l.updated("a", Some(""), Some(false)),
            Some(Event::Disconnected("Buds Pro".into()))
        );
        assert_eq!(l.updated("zzz", None, Some(true)), None);
    }

    #[test]
    fn a_new_pairing_tells_if_connected() {
        let mut l = Links::default();
        l.ready();
        assert_eq!(
            l.added("b", "Mouse", true),
            Some(Event::Connected("Mouse".into()))
        );
        assert_eq!(l.added("c", "Keys", false), None);
        l.removed("b");
        assert_eq!(l.updated("b", None, Some(false)), None);
    }
}
