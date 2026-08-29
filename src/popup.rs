use crate::{NItem, Popup};
use slint::{ComponentHandle, PhysicalPosition, Timer, TimerMode, Weak};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

// 弹窗队列管理：同一时刻最多显示一个弹窗，时序由 deadline + 事件泵驱动
pub struct PopupMgr {
    weak: Weak<Popup>,
    queue: VecDeque<NItem>,
    shown: bool,
    delay: Duration,
    maxq: usize,
    deadline: Option<Instant>,
    enter: Timer,
}

impl PopupMgr {
    pub fn new(weak: Weak<Popup>, secs: u64, maxq: usize) -> Self {
        Self {
            weak,
            queue: VecDeque::new(),
            shown: false,
            delay: Duration::from_secs(secs.max(1)),
            maxq,
            deadline: None,
            enter: Timer::default(),
        }
    }

    pub fn push(&mut self, it: NItem) {
        if self.shown {
            if self.queue.len() < self.maxq {
                self.queue.push_back(it);
            }
        } else {
            self.show(it);
        }
    }

    // 关闭当前并展示下一条
    pub fn advance(&mut self) {
        self.shown = false;
        self.deadline = None;
        if let Some(p) = self.weak.upgrade() {
            let _ = p.hide();
        }
        if let Some(it) = self.queue.pop_front() {
            self.show(it);
        }
    }

    // 到达自动消失时间
    pub fn tick(&mut self) -> bool {
        match self.deadline {
            Some(d) if Instant::now() >= d => {
                self.deadline = None;
                true
            }
            _ => false,
        }
    }

    fn show(&mut self, it: NItem) {
        let Some(p) = self.weak.upgrade() else { return };
        p.set_cur(it);
        p.set_shown(false);
        let _ = p.show();
        place(&p);
        let w = self.weak.clone();
        self.enter.start(TimerMode::SingleShot, Duration::from_millis(60), move || {
            if let Some(p) = w.upgrade() {
                p.set_shown(true);
            }
        });
        self.shown = true;
        self.deadline = Some(Instant::now() + self.delay);
    }
}

// 弹窗逻辑尺寸（与 popup.slint 一致）
const PW: f64 = 384.0;
const PH: f64 = 136.0;

// 放置到屏幕右下角（任务栏上方留白）
fn place(p: &Popup) {
    let (sw, sh) = screen();
    let k = p.window().scale_factor() as f64;
    let m = 16.0 * k;
    let taskbar = 56.0 * k;
    p.window().set_position(PhysicalPosition {
        x: (sw as f64 - PW * k - m) as i32,
        y: (sh as f64 - PH * k - m - taskbar) as i32,
    });
}

#[cfg(windows)]
fn screen() -> (i32, i32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
}

#[cfg(not(windows))]
fn screen() -> (i32, i32) {
    (1920, 1080)
}
