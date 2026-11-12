//! The system's volume, as Core Audio tells it: a callback on each change
//! (from the keys, the taskbar or any app), posted to the window. Nothing
//! is polled; while the volume stays put, nothing runs.
//!
//! The callbacks come on Core Audio's own threads, so they only post a
//! message: the window's thread reads it and, when the default speakers
//! change, follows them.

use island_core::glance;
use windows::Win32::Foundation::{HWND, LPARAM, PROPERTYKEY, WPARAM};
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, EDataFlow, ERole, IMMDeviceEnumerator,
    IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator, eConsole, eRender,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CLSCTX_INPROC_SERVER, CoCreateInstance, STGM_READ};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::core::{PCWSTR, Result, implement};

/// Where the callbacks post. A window handle is just a number to them.
#[derive(Clone, Copy)]
struct Post {
    hwnd: isize,
    msg: u32,
}

impl Post {
    fn send(&self, wp: usize, lp: isize) {
        unsafe {
            let _ = PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                self.msg,
                WPARAM(wp),
                LPARAM(lp),
            );
        }
    }
}

#[implement(IAudioEndpointVolumeCallback)]
struct OnVolume(Post);

impl IAudioEndpointVolumeCallback_Impl for OnVolume_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> Result<()> {
        if let Some(d) = unsafe { data.as_ref() } {
            self.0.send(
                d.fMasterVolume.to_bits() as usize,
                d.bMuted.as_bool() as isize,
            );
        }
        Ok(())
    }
}

#[implement(IMMNotificationClient)]
struct OnDevice(Post);

impl IMMNotificationClient_Impl for OnDevice_Impl {
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, _: &PCWSTR) -> Result<()> {
        if flow == eRender && role == eConsole {
            self.0.send(0, 0);
        }
        Ok(())
    }
    fn OnDeviceStateChanged(&self, _: &PCWSTR, _: DEVICE_STATE) -> Result<()> {
        Ok(())
    }
    fn OnDeviceAdded(&self, _: &PCWSTR) -> Result<()> {
        Ok(())
    }
    fn OnDeviceRemoved(&self, _: &PCWSTR) -> Result<()> {
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _: &PCWSTR, _: &PROPERTYKEY) -> Result<()> {
        Ok(())
    }
}

/// The device's enumerator (`PKEY_Device_EnumeratorName`): `BTHENUM` or
/// `BTHLEDEVICE` for Bluetooth, `USB`, `HDAUDIO`… for the rest.
const ENUMERATOR: PROPERTYKEY = PROPERTYKEY {
    fmtid: windows::core::GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
    pid: 24,
};

/// Whether the default speakers are Bluetooth ones (their sound comes out
/// later than the player's position says). Read once per change of them.
pub fn on_bluetooth() -> bool {
    let read = || -> Result<bool> {
        unsafe {
            let devices: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)?;
            let store = devices
                .GetDefaultAudioEndpoint(eRender, eConsole)?
                .OpenPropertyStore(STGM_READ)?;
            let v = store.GetValue(&ENUMERATOR)?;
            Ok(v.to_string().to_ascii_uppercase().starts_with("BTH"))
        }
    };
    read().unwrap_or(false)
}

/// A volume message's level (0 to 1) and mute.
pub fn read(wp: WPARAM, lp: LPARAM) -> (f32, bool) {
    (f32::from_bits(wp.0 as u32), lp.0 != 0)
}

pub struct Volume {
    devices: IMMDeviceEnumerator,
    on_device: IMMNotificationClient,
    on_volume: IAudioEndpointVolumeCallback,
    /// The default speakers' volume, while there are some.
    endpoint: Option<IAudioEndpointVolume>,
}

impl Volume {
    /// Changes of the volume go to `hwnd` as `volume_msg`; a change of the
    /// default speakers as `device_msg` (then call [`Volume::follow`]).
    pub fn start(hwnd: HWND, volume_msg: u32, device_msg: u32) -> Result<Self> {
        let at = |msg| Post {
            hwnd: hwnd.0 as isize,
            msg,
        };
        unsafe {
            let devices: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)?;
            let on_device: IMMNotificationClient = OnDevice(at(device_msg)).into();
            devices.RegisterEndpointNotificationCallback(&on_device)?;
            let mut v = Self {
                devices,
                on_device,
                on_volume: OnVolume(at(volume_msg)).into(),
                endpoint: None,
            };
            v.follow();
            Ok(v)
        }
    }

    /// Listens to the default speakers now (none: nothing to listen to).
    pub fn follow(&mut self) {
        self.drop_endpoint();
        self.endpoint = unsafe {
            self.devices
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .and_then(|d| d.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None))
                .and_then(|e| e.RegisterControlChangeNotify(&self.on_volume).map(|()| e))
                .ok()
        };
    }

    /// The volume and mute now, to compare the next change with.
    pub fn now(&self) -> Option<(f32, bool)> {
        let e = self.endpoint.as_ref()?;
        unsafe {
            Some((
                e.GetMasterVolumeLevelScalar().ok()?,
                e.GetMute().ok()?.as_bool(),
            ))
        }
    }

    /// A press of a volume key, done as Windows does it: up or down
    /// ([`glance::volume_step`]), unmuting first, or the mute switched
    /// (`up: None`). The change comes back as any other. `false` when
    /// nothing moved (at 0 or 100 already, or no speakers).
    pub fn press(&self, up: Option<bool>) -> bool {
        let Some(e) = &self.endpoint else {
            return false;
        };
        let Some((level, muted)) = self.now() else {
            return false;
        };
        unsafe {
            match up {
                None => e.SetMute(!muted, std::ptr::null()).is_ok(),
                Some(up) => {
                    let to = glance::volume_step(level, up);
                    let unmuted = muted && e.SetMute(false, std::ptr::null()).is_ok();
                    let moved = glance::percent(to) != glance::percent(level)
                        && e.SetMasterVolumeLevelScalar(to, std::ptr::null()).is_ok();
                    unmuted || moved
                }
            }
        }
    }

    fn drop_endpoint(&mut self) {
        if let Some(e) = self.endpoint.take() {
            unsafe {
                let _ = e.UnregisterControlChangeNotify(&self.on_volume);
            }
        }
    }
}

impl Drop for Volume {
    fn drop(&mut self) {
        self.drop_endpoint();
        unsafe {
            let _ = self
                .devices
                .UnregisterEndpointNotificationCallback(&self.on_device);
        }
    }
}
