//! macOS system audio-device discovery and default routing.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
pub struct AudioDevices {
    pub supported: bool,
    pub inputs: Vec<AudioDevice>,
    pub outputs: Vec<AudioDevice>,
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{AudioDevice, AudioDevices};
    use std::ffi::{c_char, c_void, CStr};
    use std::ptr;

    type AudioObjectId = u32;
    type OsStatus = i32;
    type CfStringRef = *const c_void;

    const SYSTEM_OBJECT: AudioObjectId = 1;
    const GLOBAL: u32 = u32::from_be_bytes(*b"glob");
    const INPUT: u32 = u32::from_be_bytes(*b"inpt");
    const OUTPUT: u32 = u32::from_be_bytes(*b"outp");
    const MASTER: u32 = 0;
    const DEVICES: u32 = u32::from_be_bytes(*b"dev#");
    const DEFAULT_INPUT: u32 = u32::from_be_bytes(*b"dIn ");
    const DEFAULT_OUTPUT: u32 = u32::from_be_bytes(*b"dOut");
    const NAME: u32 = u32::from_be_bytes(*b"lnam");
    const STREAMS: u32 = u32::from_be_bytes(*b"stm#");
    const UTF8: u32 = 0x0800_0100;

    #[repr(C)]
    struct PropertyAddress {
        selector: u32,
        scope: u32,
        element: u32,
    }

    #[link(name = "CoreAudio", kind = "framework")]
    extern "C" {
        fn AudioObjectGetPropertyDataSize(
            object: AudioObjectId,
            address: *const PropertyAddress,
            qualifier_size: u32,
            qualifier_data: *const c_void,
            data_size: *mut u32,
        ) -> OsStatus;
        fn AudioObjectGetPropertyData(
            object: AudioObjectId,
            address: *const PropertyAddress,
            qualifier_size: u32,
            qualifier_data: *const c_void,
            data_size: *mut u32,
            data: *mut c_void,
        ) -> OsStatus;
        fn AudioObjectSetPropertyData(
            object: AudioObjectId,
            address: *const PropertyAddress,
            qualifier_size: u32,
            qualifier_data: *const c_void,
            data_size: u32,
            data: *const c_void,
        ) -> OsStatus;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFStringGetLength(value: CfStringRef) -> isize;
        fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
        fn CFStringGetCString(
            value: CfStringRef,
            buffer: *mut c_char,
            buffer_size: isize,
            encoding: u32,
        ) -> bool;
        fn CFRelease(value: *const c_void);
    }

    fn address(selector: u32, scope: u32) -> PropertyAddress {
        PropertyAddress { selector, scope, element: MASTER }
    }

    fn read_scalar(object: AudioObjectId, selector: u32) -> Result<u32, String> {
        let mut value = 0_u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        let status = unsafe {
            AudioObjectGetPropertyData(
                object,
                &address(selector, GLOBAL),
                0,
                ptr::null(),
                &mut size,
                (&mut value as *mut u32).cast(),
            )
        };
        (status == 0)
            .then_some(value)
            .ok_or_else(|| format!("CoreAudio 读取失败 ({status})"))
    }

    fn device_ids() -> Result<Vec<AudioObjectId>, String> {
        let addr = address(DEVICES, GLOBAL);
        let mut size = 0_u32;
        let status = unsafe {
            AudioObjectGetPropertyDataSize(SYSTEM_OBJECT, &addr, 0, ptr::null(), &mut size)
        };
        if status != 0 {
            return Err(format!("CoreAudio 无法枚举设备 ({status})"));
        }
        let count = size as usize / std::mem::size_of::<AudioObjectId>();
        let mut ids = vec![0_u32; count];
        let status = unsafe {
            AudioObjectGetPropertyData(
                SYSTEM_OBJECT,
                &addr,
                0,
                ptr::null(),
                &mut size,
                ids.as_mut_ptr().cast(),
            )
        };
        if status == 0 { Ok(ids) } else { Err(format!("CoreAudio 无法枚举设备 ({status})")) }
    }

    fn has_streams(device: AudioObjectId, scope: u32) -> bool {
        let mut size = 0_u32;
        unsafe {
            AudioObjectGetPropertyDataSize(
                device,
                &address(STREAMS, scope),
                0,
                ptr::null(),
                &mut size,
            ) == 0
                && size > 0
        }
    }

    fn device_name(device: AudioObjectId) -> Option<String> {
        let mut value: CfStringRef = ptr::null();
        let mut size = std::mem::size_of::<CfStringRef>() as u32;
        let status = unsafe {
            AudioObjectGetPropertyData(
                device,
                &address(NAME, GLOBAL),
                0,
                ptr::null(),
                &mut size,
                (&mut value as *mut CfStringRef).cast(),
            )
        };
        if status != 0 || value.is_null() {
            return None;
        }
        let result = unsafe {
            let length = CFStringGetLength(value);
            let capacity = CFStringGetMaximumSizeForEncoding(length, UTF8) + 1;
            let mut bytes = vec![0_i8; capacity.max(1) as usize];
            let ok = CFStringGetCString(value, bytes.as_mut_ptr(), capacity, UTF8);
            let name = ok.then(|| CStr::from_ptr(bytes.as_ptr()).to_string_lossy().into_owned());
            CFRelease(value);
            name
        };
        result
    }

    pub fn list() -> Result<AudioDevices, String> {
        let default_input = read_scalar(SYSTEM_OBJECT, DEFAULT_INPUT).unwrap_or(0);
        let default_output = read_scalar(SYSTEM_OBJECT, DEFAULT_OUTPUT).unwrap_or(0);
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for id in device_ids()? {
            let Some(name) = device_name(id) else { continue };
            if has_streams(id, INPUT) {
                inputs.push(AudioDevice { id, name: name.clone(), is_default: id == default_input });
            }
            if has_streams(id, OUTPUT) {
                outputs.push(AudioDevice { id, name, is_default: id == default_output });
            }
        }
        inputs.sort_by(|a, b| a.name.cmp(&b.name));
        outputs.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(AudioDevices { supported: true, inputs, outputs })
    }

    pub fn set(kind: &str, device: u32) -> Result<(), String> {
        let selector = match kind {
            "input" => DEFAULT_INPUT,
            "output" => DEFAULT_OUTPUT,
            _ => return Err("音频设备类型无效".into()),
        };
        let status = unsafe {
            AudioObjectSetPropertyData(
                SYSTEM_OBJECT,
                &address(selector, GLOBAL),
                0,
                ptr::null(),
                std::mem::size_of::<u32>() as u32,
                (&device as *const u32).cast(),
            )
        };
        if status == 0 { Ok(()) } else { Err(format!("切换音频设备失败 ({status})")) }
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn lists_host_audio_devices() {
            let devices = super::list().expect("CoreAudio device enumeration");
            // Headless/sandboxed test runners may expose no host audio
            // hardware. Enumeration itself must still succeed cleanly.
            assert!(devices.inputs.iter().filter(|d| d.is_default).count() <= 1);
            assert!(devices.outputs.iter().filter(|d| d.is_default).count() <= 1);
        }
    }
}

pub fn list() -> Result<AudioDevices, String> {
    #[cfg(target_os = "macos")]
    return macos::list();

    #[cfg(not(target_os = "macos"))]
    Ok(AudioDevices { supported: false, inputs: Vec::new(), outputs: Vec::new() })
}

pub fn set(kind: &str, device: u32) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return macos::set(kind, device);

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (kind, device);
        Err("当前系统暂不支持内置音频路由".into())
    }
}
