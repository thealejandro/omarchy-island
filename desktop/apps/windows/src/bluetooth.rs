//! Paired Bluetooth devices connecting and disconnecting, from the system's
//! device watcher (only paired devices: it never scans for new ones). Its
//! events come on the system's threads; they keep the list (see
//! `island_core::bluetooth`) and post the window to read what changed.

use std::sync::{Arc, Mutex};

use island_core::bluetooth::{Event, Links};
use windows::Devices::Enumeration::{
    DeviceInformation, DeviceInformationKind, DeviceInformationUpdate, DeviceWatcher,
};
use windows::Foundation::{IReference, TypedEventHandler};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::core::{HSTRING, IInspectable, Interface, Result};
use windows_collections::{IIterable, IMapView};

/// Paired devices, classic or Low Energy.
const FILTER: &str = "(System.Devices.Aep.ProtocolId:=\"{e0cbf06c-cd8b-4647-bb8a-263b43f0f974}\" \
     OR System.Devices.Aep.ProtocolId:=\"{bb7bb05e-5972-42b5-94fc-76eaa7084d49}\") \
     AND System.Devices.Aep.IsPaired:=System.StructuredQueryType.Boolean#True";
const CONNECTED: &str = "System.Devices.Aep.IsConnected";

#[derive(Default)]
struct Shared {
    links: Links,
    /// Told, not yet read by the window.
    events: Vec<Event>,
}

fn connected(props: &IMapView<HSTRING, IInspectable>) -> Option<bool> {
    let v = props.Lookup(&HSTRING::from(CONNECTED)).ok()?;
    v.cast::<IReference<bool>>().ok()?.Value().ok()
}

pub struct Bluetooth {
    watcher: DeviceWatcher,
    shared: Arc<Mutex<Shared>>,
}

impl Bluetooth {
    /// Each change goes to `hwnd` as `msg`; then call [`Bluetooth::take`].
    pub fn start(hwnd: HWND, msg: u32) -> Result<Self> {
        let props: IIterable<HSTRING> = vec![HSTRING::from(CONNECTED)].into();
        let watcher = DeviceInformation::CreateWatcherWithKindAqsFilterAndAdditionalProperties(
            &HSTRING::from(FILTER),
            &props,
            DeviceInformationKind::AssociationEndpoint,
        )?;
        let shared = Arc::new(Mutex::new(Shared::default()));
        let hwnd = hwnd.0 as isize;
        // Keeps `e` for the window and wakes it.
        let tell = move |s: &mut Shared, e: Option<Event>| {
            if let Some(e) = e {
                s.events.push(e);
                unsafe {
                    let _ = PostMessageW(Some(HWND(hwnd as *mut _)), msg, WPARAM(0), LPARAM(0));
                }
            }
        };

        let s = shared.clone();
        watcher.Added(&TypedEventHandler::<DeviceWatcher, DeviceInformation>::new(
            move |_, d| {
                if let (Some(d), Ok(mut s)) = (d.as_ref(), s.lock()) {
                    let on = d.Properties().ok().and_then(|p| connected(&p));
                    let e = s.links.added(
                        &d.Id()?.to_string(),
                        &d.Name()?.to_string(),
                        on.unwrap_or(false),
                    );
                    tell(&mut s, e);
                }
                Ok(())
            },
        ))?;
        let s = shared.clone();
        watcher.Updated(
            &TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(move |_, u| {
                if let (Some(u), Ok(mut s)) = (u.as_ref(), s.lock()) {
                    let on = u.Properties().ok().and_then(|p| connected(&p));
                    let e = s.links.updated(&u.Id()?.to_string(), None, on);
                    tell(&mut s, e);
                }
                Ok(())
            }),
        )?;
        let s = shared.clone();
        watcher.Removed(
            &TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(move |_, u| {
                if let (Some(u), Ok(mut s)) = (u.as_ref(), s.lock()) {
                    s.links.removed(&u.Id()?.to_string());
                }
                Ok(())
            }),
        )?;
        let s = shared.clone();
        watcher.EnumerationCompleted(&TypedEventHandler::<DeviceWatcher, IInspectable>::new(
            move |_, _| {
                if let Ok(mut s) = s.lock() {
                    s.links.ready();
                }
                Ok(())
            },
        ))?;
        watcher.Start()?;
        Ok(Self { watcher, shared })
    }

    /// What was told since the last call.
    pub fn take(&self) -> Vec<Event> {
        self.shared
            .lock()
            .map(|mut s| std::mem::take(&mut s.events))
            .unwrap_or_default()
    }
}

impl Drop for Bluetooth {
    fn drop(&mut self) {
        let _ = self.watcher.Stop();
    }
}
