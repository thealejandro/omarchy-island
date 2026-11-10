//! The system's volume, as Core Audio tells it: a callback on each change
//! (from the keys, the taskbar or any app), posted to the window. Nothing
//! is polled; while the volume stays put, nothing runs.
//!
//! The callbacks come on Core Audio's own threads, so they only post a
//! message: the window's thread reads it and, when the default speakers
//! change, follows them.

use windows::Win32::Foundation::{HWND, LPARAM, PROPERTYKEY, WPARAM};
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, EDataFlow, ERole, IMMDeviceEnumerator,
    IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator, eConsole, eRender,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CLSCTX_INPROC_SERVER, CoCreateInstance};
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
