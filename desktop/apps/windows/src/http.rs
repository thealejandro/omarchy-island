//! HTTPS GETs through WinHTTP, for the features that go online when asked
//! (lyrics). Blocking: called from a worker thread, never the window's.

use std::ffi::c_void;

use windows::Win32::Networking::WinHttp::{
    INTERNET_DEFAULT_HTTPS_PORT, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect,
    WinHttpOpen, WinHttpOpenRequest, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse,
    WinHttpSendRequest, WinHttpSetTimeouts,
};
use windows::core::{HRESULT, HSTRING, PCWSTR, w};

/// Why a GET gave no answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetError {
    /// No way to the server: no network, no name, refused or timed out.
    Offline,
    /// Anything else: TLS, a cut answer, or a body bigger than asked for.
    Failed,
}

/// WinHTTP's errors that mean the server can't be reached.
const OFFLINE: [u32; 4] = [
    12002, // ERROR_WINHTTP_TIMEOUT
    12007, // ERROR_WINHTTP_NAME_NOT_RESOLVED
    12029, // ERROR_WINHTTP_CANNOT_CONNECT
    12030, // ERROR_WINHTTP_CONNECTION_ERROR
];

fn net_error(e: windows::core::Error) -> NetError {
    if OFFLINE.iter().any(|&n| e.code() == HRESULT::from_win32(n)) {
        NetError::Offline
    } else {
        NetError::Failed
    }
}

/// Closes a WinHTTP handle when it goes.
struct Handle(*mut c_void);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = WinHttpCloseHandle(self.0);
            }
        }
    }
}

/// A handle WinHTTP gave, or the error it set.
fn handle(h: *mut c_void) -> Result<Handle, NetError> {
    if h.is_null() {
        Err(net_error(windows::core::Error::from_thread()))
    } else {
        Ok(Handle(h))
    }
}

/// GET `https://<host><path>`: the status and at most `cap` bytes of body.
pub fn get(host: &str, path: &str, cap: usize) -> Result<(u32, Vec<u8>), NetError> {
    let agent = HSTRING::from(format!(
        "island-windows/{} (https://github.com/thealejandro/island)",
        env!("CARGO_PKG_VERSION")
    ));
    let host = HSTRING::from(host);
    let path = HSTRING::from(path);
    unsafe {
        let session = handle(WinHttpOpen(
            &agent,
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        ))?;
        // Resolve, connect, send, receive: as the lyrics script's 4 s and 8 s.
        WinHttpSetTimeouts(session.0, 4000, 4000, 8000, 8000).map_err(net_error)?;
        let connect = handle(WinHttpConnect(
            session.0,
            &host,
            INTERNET_DEFAULT_HTTPS_PORT,
            0,
        ))?;
        // HTTPS only; WinHTTP never follows a redirect down to HTTP.
        let request = handle(WinHttpOpenRequest(
            connect.0,
            w!("GET"),
            &path,
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        ))?;
        WinHttpSendRequest(request.0, None, None, 0, 0, 0).map_err(net_error)?;
        WinHttpReceiveResponse(request.0, std::ptr::null_mut()).map_err(net_error)?;
        let mut code = 0u32;
        let mut size = size_of::<u32>() as u32;
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some(&mut code as *mut u32 as *mut c_void),
            &mut size,
            std::ptr::null_mut(),
        )
        .map_err(net_error)?;
        let mut body = Vec::new();
        let mut chunk = vec![0u8; 16 * 1024];
        loop {
            let mut read = 0u32;
            WinHttpReadData(
                request.0,
                chunk.as_mut_ptr() as *mut c_void,
                chunk.len() as u32,
                &mut read,
            )
            .map_err(net_error)?;
            if read == 0 {
                break;
            }
            body.extend_from_slice(&chunk[..read as usize]);
            if body.len() > cap {
                return Err(NetError::Failed);
            }
        }
        Ok((code, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_offline_from_other_failures() {
        let e = |n| windows::core::Error::from_hresult(HRESULT::from_win32(n));
        for n in OFFLINE {
            assert_eq!(net_error(e(n)), NetError::Offline);
        }
        // ERROR_WINHTTP_SECURE_FAILURE: reached, but not to be trusted.
        assert_eq!(net_error(e(12175)), NetError::Failed);
    }
}
