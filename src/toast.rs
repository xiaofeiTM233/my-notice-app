use crate::config::{Corner, Toast as Cfg};
use crate::model::Notif;
use crate::{app, win32, Theme, ToastWindow};
use slint::{ComponentHandle, PhysicalPosition, Timer, TimerMode};
use std::collections::VecDeque;
use std::time::Duration;

/// 与 toast.slint 中 pad 默认值保持一致：窗口比卡片大一圈，用来放阴影
const PAD: f32 = 18.0;

struct Slot {
    win: ToastWindow,
    id: String,
    life: Timer,
    fade: Timer,
}

pub struct Toasts {
    cfg: Cfg,
    dark: bool,
    slots: Vec<Slot>,
    wait: VecDeque<Notif>,
}

impl Toasts {
    pub fn new(cfg: Cfg, dark: bool) -> Self {
        Toasts {
            cfg,
            dark,
            slots: vec![],
            wait: VecDeque::new(),
        }
    }

    pub fn set_dark(&mut self, dark: bool) {
        self.dark = dark;
        for s in &self.slots {
            s.win.global::<Theme>().set_dark(dark);
        }
    }

    pub fn push(&mut self, n: Notif) {
        if !self.cfg.enabled {
            return;
        }
        if self.slots.len() >= self.cfg.max_visible.max(1) {
            if self.wait.len() < 64 {
                self.wait.push_back(n);
            }
            return;
        }
        if let Err(e) = self.open(n) {
            eprintln!("toast 创建失败: {e}");
        }
    }

    fn open(&mut self, n: Notif) -> Result<(), slint::PlatformError> {
        let w = ToastWindow::new()?;
        w.global::<Theme>().set_dark(self.dark);

        w.set_head(n.title.as_str().into());
        w.set_body(n.body.as_str().into());
        w.set_src(n.src.as_str().into());
        w.set_tag(n.cat.label().into());
        w.set_mark(n.cat.mark().into());
        w.set_cat(n.cat.idx());
        w.set_prio(n.prio.idx());
        w.set_act(n.act.is_some());
        w.set_cw(self.cfg.width);
        w.set_ch(self.cfg.height);
        w.set_pad(PAD);
        w.set_anim(self.cfg.anim_ms);
        w.set_dwell(self.cfg.duration_ms as i64);

        let id = n.id.clone();
        let a = id.clone();
        w.on_opened(move || {
            let id = a.clone();
            app::with(|x| x.toast_click(&id));
        });
        let b = id.clone();
        w.on_closed(move || {
            let id = b.clone();
            app::with(|x| x.toast_close(&id));
        });
        let c = id.clone();
        let hold = self.cfg.hold_on_hover;
        w.on_hovered(move |on| {
            if on && hold {
                let id = c.clone();
                app::with(|x| x.toast_hold(&id));
            }
        });

        w.show()?;
        win32::as_toast(w.window());
        self.locate(&w, self.slots.len());

        w.set_shown(true);
        w.set_counting(true);

        let life = Timer::default();
        let d = id.clone();
        life.start(
            TimerMode::SingleShot,
            Duration::from_millis(self.cfg.duration_ms),
            move || {
                let id = d.clone();
                app::with(|x| x.toast_close(&id));
            },
        );

        self.slots.push(Slot {
            win: w,
            id,
            life,
            fade: Timer::default(),
        });
        Ok(())
    }

    /// 悬停时取消自动关闭
    pub fn hold(&self, id: &str) {
        if let Some(s) = self.slots.iter().find(|s| s.id == id) {
            s.life.stop();
        }
    }

    pub fn close(&mut self, id: &str) {
        let Some(i) = self.slots.iter().position(|s| s.id == id) else {
            return;
        };
        let s = &self.slots[i];
        s.life.stop();
        s.win.set_shown(false);
        let gone = s.id.clone();
        s.fade.start(
            TimerMode::SingleShot,
            Duration::from_millis(self.cfg.anim_ms.max(0) as u64 + 20),
            move || {
                let id = gone.clone();
                app::with(|x| x.toast_drop(&id));
            },
        );
    }

    /// 动画结束后真正销毁窗口并补位
    pub fn drop_win(&mut self, id: &str) {
        let Some(i) = self.slots.iter().position(|s| s.id == id) else {
            return;
        };
        let s = self.slots.remove(i);
        let _ = s.win.hide();
        drop(s);

        for (i, s) in self.slots.iter().enumerate() {
            self.locate(&s.win, i);
        }
        if let Some(n) = self.wait.pop_front() {
            let _ = self.open(n);
        }
    }

    pub fn close_all(&mut self) {
        self.wait.clear();
        for s in self.slots.drain(..) {
            let _ = s.win.hide();
        }
    }

    /// slot 0 贴近屏幕角，序号越大越远离
    fn locate(&self, w: &ToastWindow, i: usize) {
        let a = win32::work_area();
        let sf = w.window().scale_factor().max(0.1);
        let (cw, ch) = (self.cfg.width, self.cfg.height);
        let (m, g) = (self.cfg.margin, self.cfg.gap);
        let step = (ch + g) * i as f32;

        let (l, t, r, b) = (
            a.l as f32 / sf,
            a.t as f32 / sf,
            a.r as f32 / sf,
            a.b as f32 / sf,
        );

        let x = match self.cfg.corner {
            Corner::BottomRight | Corner::TopRight => r - m - cw - PAD,
            Corner::BottomLeft | Corner::TopLeft => l + m - PAD,
        };
        let y = match self.cfg.corner {
            Corner::BottomRight | Corner::BottomLeft => b - m - step - ch - PAD,
            Corner::TopRight | Corner::TopLeft => t + m + step - PAD,
        };

        w.window().set_position(PhysicalPosition::new(
            (x * sf).round() as i32,
            (y * sf).round() as i32,
        ));
    }
}
