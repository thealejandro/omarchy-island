//! Paired Bluetooth devices connecting and disconnecting, from the system's
//! device watcher (only paired devices: it never scans for new ones). Its
//! events come on the system's threads; they keep the list (see
//! `island_core::bluetooth`) and post the window, which reads what held.

use std::sync::{Arc, Mutex};
use std::time::Instant;

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

fn connected(props: &IMapView<HSTRING, IInspectable>) -> Option<bool> {
    let v = props.Lookup(&HSTRING::from(CONNECTED)).ok()?;
    v.cast::<IReference<bool>>().ok()?.Value().ok()
}

pub struct Bluetooth {
    watcher: DeviceWatcher,
    links: Arc<Mutex<Links>>,
    /// The clock the list's times are on.
    start: Instant,
}

impl Bluetooth {
    /// Each change goes to `hwnd` as `msg`; then call [`Bluetooth::due`].
    pub fn start(hwnd: HWND, msg: u32) -> Result<Self> {
        let props: IIterable<HSTRING> = vec![HSTRING::from(CONNECTED)].into();
        let watcher = DeviceInformation::CreateWatcherWithKindAqsFilterAndAdditionalProperties(
            &HSTRING::from(FILTER),
            &props,
            DeviceInformationKind::AssociationEndpoint,
        )?;
        let links = Arc::new(Mutex::new(Links::default()));
        let start = Instant::now();
        let hwnd = hwnd.0 as isize;
        let at = move || start.elapsed().as_millis() as u64;
        let wake = move || unsafe {
            let _ = PostMessageW(Some(HWND(hwnd as *mut _)), msg, WPARAM(0), LPARAM(0));
        };

        let l = links.clone();
        watcher.Added(&TypedEventHandler::<DeviceWatcher, DeviceInformation>::new(
            move |_, d| {
                if let (Some(d), Ok(mut l)) = (d.as_ref(), l.lock()) {
                    let on = d.Properties().ok().and_then(|p| connected(&p));
                    let (id, name) = (d.Id()?.to_string(), d.Name()?.to_string());
                    l.added(&id, &name, on.unwrap_or(false), at());
                    wake();
                }
                Ok(())
            },
        ))?;
        let l = links.clone();
        watcher.Updated(
            &TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(move |_, u| {
                if let (Some(u), Ok(mut l)) = (u.as_ref(), l.lock()) {
                    let on = u.Properties().ok().and_then(|p| connected(&p));
                    if on.is_some() {
                        l.updated(&u.Id()?.to_string(), None, on, at());
                        wake();
                    }
                }
                Ok(())
            }),
        )?;
        let l = links.clone();
        watcher.Removed(
            &TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(move |_, u| {
                if let (Some(u), Ok(mut l)) = (u.as_ref(), l.lock()) {
                    l.removed(&u.Id()?.to_string());
                }
                Ok(())
            }),
        )?;
        let l = links.clone();
        watcher.EnumerationCompleted(&TypedEventHandler::<DeviceWatcher, IInspectable>::new(
            move |_, _| {
                if let Ok(mut l) = l.lock() {
                    l.ready();
                }
                Ok(())
            },
        ))?;
        watcher.Start()?;
        Ok(Self {
            watcher,
            links,
            start,
        })
    }

    /// The changes that have held since the last call, and in how many ms
    /// to ask again for one still settling.
    pub fn due(&self) -> (Vec<Event>, Option<u64>) {
        let at = self.start.elapsed().as_millis() as u64;
        self.links.lock().map(|mut l| l.due(at)).unwrap_or_default()
    }
}

impl Drop for Bluetooth {
    fn drop(&mut self) {
        let _ = self.watcher.Stop();
    }
}
