#[derive(Clone, Copy, Debug)]
pub struct Area {
    pub l: i32,
    pub t: i32,
    pub r: i32,
    pub b: i32,
}

#[cfg(windows)]
mod imp {
    use super::Area;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, GetWindowLongPtrW, SetWindowLongPtrW, SystemParametersInfoW, GWL_EXSTYLE,
        SM_CXSCREEN, SM_CYSCREEN, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    /// 主显示器工作区（物理像素，已排除任务栏）
    pub fn work_area() -> Area {
        unsafe {
            let mut r = RECT::default();
            let ok = SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some(&mut r as *mut RECT as *mut core::ffi::c_void),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );
            if ok.is_ok() && r.right > r.left && r.bottom > r.top {
                return Area {
                    l: r.left,
                    t: r.top,
                    r: r.right,
                    b: r.bottom,
                };
            }
            Area {
                l: 0,
                t: 0,
                r: GetSystemMetrics(SM_CXSCREEN),
                b: GetSystemMetrics(SM_CYSCREEN),
            }
        }
    }

    fn handle(w: &slint::Window) -> Option<HWND> {
        let owner = w.window_handle();
        let h = owner.window_handle().ok()?;
        match h.as_raw() {
            RawWindowHandle::Win32(x) => Some(HWND(x.hwnd.get() as *mut core::ffi::c_void)),
            _ => None,
        }
    }

    /// 让弹窗不占任务栏、不抢焦点。需在 show() 之后调用。
    pub fn as_toast(w: &slint::Window) {
        let Some(h) = handle(w) else { return };
        unsafe {
            let cur = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
            let want = cur | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0;
            if want != cur {
                SetWindowLongPtrW(h, GWL_EXSTYLE, want as isize);
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Area;
    pub fn work_area() -> Area {
        Area {
            l: 0,
            t: 0,
            r: 1920,
            b: 1040,
        }
    }
    pub fn as_toast(_w: &slint::Window) {}
}

pub use imp::{as_toast, work_area};
