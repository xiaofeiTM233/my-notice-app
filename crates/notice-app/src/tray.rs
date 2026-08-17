//! 系统托盘（仅 Windows）。
//!
//! gpui 0.2.2 只提供 macOS 的托盘支持，Windows 平台通过 Win32 `Shell_NotifyIconW`
//! 直接实现：
//!
//! - 创建隐藏消息窗口接收托盘回调；
//! - 左键单击托盘图标 → `SystemEvent::ShowMainWindow`；
//! - 右键 → 弹出菜单（显示主窗口 / 退出）；
//! - 通过子类化主窗口拦截 `WM_CLOSE`，实现"关闭窗口 → 驻留托盘"；
//! - `Drop` 时移除托盘图标并销毁消息窗口。

#![cfg(target_os = "windows")]

use std::ffi::c_void;
use std::sync::mpsc::Sender;

use gpui::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CREATESTRUCTW, CallWindowProcW, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyMenu, DestroyWindow, GetCursorPos, GetWindowLongPtrW, GWLP_USERDATA,
    GWLP_WNDPROC, IDI_APPLICATION, LoadIconW, MF_SEPARATOR, MF_STRING, RegisterClassExW,
    SetForegroundWindow, SetWindowLongPtrW, ShowWindow, SW_HIDE, SW_SHOW, TrackPopupMenu,
    TPM_LEFTALIGN, TPM_RIGHTBUTTON, WINDOW_EX_STYLE, WNDCLASSEXW, WNDPROC, WM_APP, WM_COMMAND,
    WM_CREATE, WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP, WS_OVERLAPPED,
};

use crate::app::SystemEvent;

const TRAY_ICON_ID: u32 = 1;
const TRAY_CALLBACK_MSG: u32 = WM_APP + 1;
const WM_CLOSE: u32 = 0x0010;
const MENU_SHOW: usize = 1;
const MENU_QUIT: usize = 2;

/// 被替换前的主窗口原始窗口过程（子类化时保存，供转发使用）。
///
/// 注意：不能在 `host_wnd_proc` 内用 `GetWindowLongPtrW` 读取，
/// 因为那时读到的已经是 `host_wnd_proc` 自身，会导致无限递归。
static ORIGINAL_WNDPROC: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

/// 托盘控制器：封装托盘实例与主窗口句柄。
pub struct TrayController {
    /// 仅用于保持存活：`Drop` 时移除托盘图标并恢复主窗口过程。
    _tray: Tray,
    hwnd: *mut c_void,
}

impl TrayController {
    /// 安装托盘：创建图标并子类化主窗口（关闭 → 隐藏到托盘）。
    ///
    /// `tx` 为系统事件发送端（与主窗口共享，托盘事件由此回传）。
    pub fn install(window: &mut Window, tx: Sender<SystemEvent>) -> anyhow::Result<Self> {
        let mut tray = Tray::create(tx)?;

        let hwnd = match window.window_handle() {
            Ok(handle) => match handle.as_raw() {
                RawWindowHandle::Win32(h) => h.hwnd.get() as *mut c_void,
                _ => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        };
        if !hwnd.is_null() {
            tray.intercept_close(hwnd);
        }
        Ok(Self {
            _tray: tray,
            hwnd,
        })
    }

    /// 显示并激活主窗口。
    pub fn show(&self) {
        if !self.hwnd.is_null() {
            show_window(self.hwnd);
        }
    }
}

/// 托盘实例。持有消息窗口句柄，`Drop` 时自动清理。
pub struct Tray {
    hwnd: HWND,
    host_hwnd: Option<HWND>,
    original_wndproc: Option<isize>,
}

impl Tray {
    /// 创建托盘图标并开始接收托盘事件。
    pub fn create(event_tx: Sender<SystemEvent>) -> anyhow::Result<Self> {
        unsafe {
            let hmodule = GetModuleHandleW(None)?;
            let hinstance = windows::Win32::Foundation::HINSTANCE(hmodule.0);
            let class_name = w!("MyNoticeTrayWindow");

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(tray_wnd_proc),
                hInstance: hinstance,
                lpszClassName: class_name,
                ..Default::default()
            };
            if RegisterClassExW(&wnd_class) == 0 {
                anyhow::bail!("RegisterClassExW 失败");
            }

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                w!("MyNoticeTray"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                hinstance,
                Some(Box::into_raw(Box::new(event_tx)) as *const c_void),
            )?;

            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: TRAY_ICON_ID,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: TRAY_CALLBACK_MSG,
                hIcon: LoadIconW(None, IDI_APPLICATION)?,
                ..Default::default()
            };
            write_wide(&mut nid.szTip, "My Notice — 通知盒");

            if !Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
                anyhow::bail!("Shell_NotifyIconW(NIM_ADD) 失败");
            }

            Ok(Self {
                hwnd,
                host_hwnd: None,
                original_wndproc: None,
            })
        }
    }

    /// 子类化宿主窗口：点击关闭按钮时隐藏到托盘而不是退出。
    pub fn intercept_close(&mut self, host_hwnd: *mut c_void) {
        unsafe {
            let hwnd = HWND(host_hwnd);
            let proc = host_wnd_proc
                as unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
            let original = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as isize);
            if original != 0 {
                ORIGINAL_WNDPROC.store(original, std::sync::atomic::Ordering::SeqCst);
                self.host_hwnd = Some(hwnd);
                self.original_wndproc = Some(original);
            }
        }
    }

    fn restore_host_wndproc(&mut self) {
        if let (Some(hwnd), Some(original)) = (self.host_hwnd.take(), self.original_wndproc.take())
        {
            unsafe {
                let _ = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, original);
            }
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        self.restore_host_wndproc();
        unsafe {
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: TRAY_ICON_ID,
                ..Default::default()
            };
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

/// 托盘消息窗口过程。
unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_CREATE => {
                // WM_CREATE 的 lParam 指向 CREATESTRUCTW，lpCreateParams 是我们传入的 tx 指针
                let create = &*(lparam.0 as *const CREATESTRUCTW);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
                LRESULT(0)
            }
            TRAY_CALLBACK_MSG => {
                let tx = window_tx(hwnd);
                match lparam.0 as u32 {
                    WM_LBUTTONUP => {
                        let _ = tx.send(SystemEvent::ShowMainWindow);
                    }
                    WM_RBUTTONUP => show_menu(hwnd),
                    _ => {}
                }
                LRESULT(0)
            }
            WM_COMMAND => {
                let tx = window_tx(hwnd);
                match wparam.0 & 0xFFFF {
                    MENU_SHOW => {
                        let _ = tx.send(SystemEvent::ShowMainWindow);
                    }
                    MENU_QUIT => {
                        let _ = tx.send(SystemEvent::Quit);
                    }
                    _ => {}
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                // 回收 WM_CREATE 时保存的 tx
                let ptr = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut Sender<SystemEvent>;
                if !ptr.is_null() {
                    drop(Box::from_raw(ptr));
                }
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// 从窗口用户数据取出 event_tx 的克隆。
unsafe fn window_tx(hwnd: HWND) -> Sender<SystemEvent> {
    unsafe {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Sender<SystemEvent>;
        if ptr.is_null() {
            let (tx, _) = std::sync::mpsc::channel();
            tx
        } else {
            (*ptr).clone()
        }
    }
}

/// 弹出右键菜单。
unsafe fn show_menu(hwnd: HWND) {
    unsafe {
        let menu = match CreatePopupMenu() {
            Ok(m) => m,
            Err(_) => return,
        };
        let _ = AppendMenuW(menu, MF_STRING, MENU_SHOW, w!("显示主窗口"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, MF_STRING, MENU_QUIT, w!("退出"));

        // 让菜单可被点击后自动关闭
        let _ = SetForegroundWindow(hwnd);
        let mut pos = POINT::default();
        if GetCursorPos(&mut pos).is_ok() {
            let _ = TrackPopupMenu(
                menu,
                TPM_LEFTALIGN | TPM_RIGHTBUTTON,
                pos.x,
                pos.y,
                0,
                hwnd,
                None,
            );
        }
        let _ = DestroyMenu(menu);
    }
}

/// 宿主窗口（主窗口）的子类化窗口过程：拦截关闭，其余转发给原过程。
unsafe extern "system" fn host_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if msg == WM_CLOSE {
            // 关闭 → 隐藏到托盘
            let _ = ShowWindow(hwnd, SW_HIDE);
            return LRESULT(0);
        }
        let original = ORIGINAL_WNDPROC.load(std::sync::atomic::Ordering::SeqCst);
        if original == 0 {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let original_proc: WNDPROC = std::mem::transmute(original);
        CallWindowProcW(original_proc, hwnd, msg, wparam, lparam)
    }
}

/// 把 UTF-8 字符串写入定长宽字符缓冲区。
fn write_wide(buf: &mut [u16], text: &str) {
    let mut it = text.encode_utf16();
    for slot in buf.iter_mut() {
        *slot = it.next().unwrap_or(0);
    }
}

/// 显示主窗口（从托盘恢复）。
pub fn show_window(hwnd: *mut c_void) {
    unsafe {
        let hwnd = HWND(hwnd);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
}
