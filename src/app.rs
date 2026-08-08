use crate::config::{self, Config};
use crate::feed::{self, Ev};
use crate::model::{Cat, Notif, Prio};
use crate::store::Store;
use crate::toast::Toasts;
use crate::{icon, view, win32, CenterWindow, Row, Theme};
use chrono::{Local, Timelike};
use slint::{ComponentHandle, PhysicalPosition, Timer, TimerMode, VecModel};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// 与 center.slint 的 pad 保持一致
const PAD: f32 = 14.0;
const SAVE_DELAY: u64 = 3000;

thread_local! {
    static CUR: RefCell<Option<Rc<App>>> = const { RefCell::new(None) };
}

pub fn install(a: Rc<App>) {
    CUR.with(|c| *c.borrow_mut() = Some(a));
}

pub fn clear() {
    CUR.with(|c| *c.borrow_mut() = None);
}

/// 在 UI 线程上取用 App
pub fn with<F: FnOnce(&App)>(f: F) {
    let a = CUR.with(|c| c.borrow().clone());
    if let Some(a) = a {
        f(&a);
    }
}

/// 从任意线程投递任务到 UI 线程
pub fn post<F: FnOnce(&App) + Send + 'static>(f: F) {
    let _ = slint::invoke_from_event_loop(move || with(f));
}

pub struct App {
    pub cfg: RefCell<Config>,
    pub store: RefCell<Store>,
    pub win: CenterWindow,
    pub toasts: RefCell<Toasts>,
    pub tray: RefCell<Option<tray_icon::TrayIcon>>,
    pub feed: RefCell<Option<feed::Handle>>,
    model: Rc<VecModel<Row>>,
    query: RefCell<String>,
    saver: Timer,
    paused: Cell<bool>,
    online: Cell<bool>,
    badge: Cell<bool>,
    placed: Cell<bool>,
}

impl App {
    pub fn new(cfg: Config, store: Store) -> Result<Rc<Self>, slint::PlatformError> {
        let win = CenterWindow::new()?;
        let model = Rc::new(VecModel::<Row>::from(Vec::<Row>::new()));

        win.set_rows(model.clone().into());
        win.set_cw(cfg.ui.width);
        win.set_ch(cfg.ui.height);
        win.set_pad(PAD);
        win.global::<Theme>().set_dark(cfg.ui.dark);

        let toasts = Toasts::new(cfg.toast.clone(), cfg.ui.dark);

        let a = Rc::new(App {
            cfg: RefCell::new(cfg),
            store: RefCell::new(store),
            win,
            toasts: RefCell::new(toasts),
            tray: RefCell::new(None),
            feed: RefCell::new(None),
            model,
            query: RefCell::new(String::new()),
            saver: Timer::default(),
            paused: Cell::new(false),
            online: Cell::new(true),
            badge: Cell::new(false),
            placed: Cell::new(false),
        });
        a.wire();
        Ok(a)
    }

    fn wire(&self) {
        let w = &self.win;
        w.on_search(|t| with(|a| a.on_search(t.as_str())));
        w.on_pick(|f| {
            with(|a| {
                a.win.set_filter(f);
                a.refresh();
            })
        });
        w.on_open(|id| with(|a| a.activate(id.as_str())));
        w.on_del(|id| with(|a| a.del(id.as_str())));
        w.on_readall(|| {
            with(|a| {
                a.store.borrow_mut().mark_all();
                a.dirty();
                a.refresh();
            })
        });
        w.on_clearall(|| {
            with(|a| {
                a.store.borrow_mut().clear();
                a.dirty();
                a.refresh();
            })
        });
        w.on_hide_win(|| with(|a| a.hide_center()));
        w.on_min_win(|| with(|a| a.win.window().set_minimized(true)));
        w.on_toggle_pause(|| with(|a| a.toggle_pause()));
        w.on_toggle_dark(|| {
            with(|a| {
                let d = !a.cfg.borrow().ui.dark;
                a.set_dark(d);
            })
        });
        w.on_drag(|dx, dy| with(|a| a.drag(dx, dy)));

        w.window()
            .on_close_requested(|| slint::CloseRequestResponse::HideWindow);
    }

    // ── 数据

    pub fn on_ev(&self, ev: Ev) {
        match ev {
            Ev::Got(n) => self.add(n, true),
            Ev::Up(ok, why) => {
                if self.online.replace(ok) == ok {
                    return;
                }
                if ok {
                    self.add(
                        Notif::local("通知服务已恢复", &why, Cat::System, Prio::Low),
                        false,
                    );
                } else {
                    self.add(
                        Notif::local("通知服务连接中断", &why, Cat::System, Prio::High),
                        true,
                    );
                }
            }
        }
    }

    pub fn add(&self, n: Notif, popup: bool) {
        let fresh = self.store.borrow_mut().push(n.clone());
        if !fresh {
            return;
        }
        if popup && !self.paused.get() {
            let t = Local::now();
            if self.cfg.borrow().filter.pass(&n, t.hour(), t.minute()) {
                self.toasts.borrow_mut().push(n);
            }
        }
        self.dirty();
        self.refresh();
    }

    fn on_search(&self, q: &str) {
        *self.query.borrow_mut() = q.to_string();
        self.refresh();
    }

    fn del(&self, id: &str) {
        self.store.borrow_mut().del(id);
        self.toasts.borrow_mut().close(id);
        self.dirty();
        self.refresh();
    }

    /// 标记已读并执行动作
    fn activate(&self, id: &str) {
        let act = self.store.borrow().get(id).and_then(|n| n.act.clone());
        self.store.borrow_mut().mark(id);
        if let Some(a) = act {
            if !a.value.is_empty() {
                let _ = open::that_detached(&a.value);
            }
        }
        self.dirty();
        self.refresh();
    }

    pub fn refresh(&self) {
        let (rows, total, unread) = {
            let s = self.store.borrow();
            let q = self.query.borrow();
            (
                view::rows(s.items(), self.win.get_filter(), &q),
                s.len() as i32,
                s.unread() as i32,
            )
        };
        self.model.set_vec(rows);
        self.win.set_total(total);
        self.win.set_unread(unread);
        self.sync_tray(unread);
    }

    fn dirty(&self) {
        self.saver.start(
            TimerMode::SingleShot,
            Duration::from_millis(SAVE_DELAY),
            || with(|a| a.store.borrow().save()),
        );
    }

    pub fn flush(&self) {
        self.saver.stop();
        self.store.borrow().save();
    }

    // ── 弹窗回调

    pub fn toast_click(&self, id: &str) {
        self.activate(id);
        self.toasts.borrow_mut().close(id);
    }

    pub fn toast_close(&self, id: &str) {
        self.toasts.borrow_mut().close(id);
    }

    pub fn toast_hold(&self, id: &str) {
        self.toasts.borrow().hold(id);
    }

    pub fn toast_drop(&self, id: &str) {
        self.toasts.borrow_mut().drop_win(id);
    }

    // ── 窗口

    pub fn show_center(&self) {
        let _ = self.win.show();
        self.place();
        self.win.window().set_minimized(false);
    }

    pub fn hide_center(&self) {
        let _ = self.win.hide();
    }

    pub fn toggle_center(&self) {
        if self.win.window().is_visible() {
            self.hide_center();
        } else {
            self.show_center();
        }
    }

    fn place(&self) {
        if self.placed.replace(true) {
            return;
        }
        let a = win32::work_area();
        let w = self.win.window();
        let sf = w.scale_factor().max(0.1);
        let (cw, ch) = {
            let c = self.cfg.borrow();
            (c.ui.width, c.ui.height)
        };
        let x = a.r as f32 / sf - 12.0 - cw - PAD;
        let y = a.b as f32 / sf - 12.0 - ch - PAD;
        w.set_position(PhysicalPosition::new(
            (x * sf).round() as i32,
            (y * sf).round() as i32,
        ));
    }

    fn drag(&self, dx: f32, dy: f32) {
        let w = self.win.window();
        let sf = w.scale_factor();
        let p = w.position();
        w.set_position(PhysicalPosition::new(
            p.x + (dx * sf).round() as i32,
            p.y + (dy * sf).round() as i32,
        ));
    }

    pub fn set_dark(&self, d: bool) {
        self.cfg.borrow_mut().ui.dark = d;
        self.win.global::<Theme>().set_dark(d);
        self.toasts.borrow_mut().set_dark(d);
        let _ = config::save(&self.cfg.borrow());
    }

    pub fn toggle_pause(&self) {
        let p = !self.paused.get();
        self.paused.set(p);
        self.win.set_paused(p);
        if p {
            self.toasts.borrow_mut().close_all();
        }
    }

    // ── 托盘

    fn sync_tray(&self, unread: i32) {
        let held = self.tray.borrow();
        let Some(tray) = held.as_ref() else { return };
        let _ = tray.set_tooltip(Some(format!("NotifyHub · 未读 {unread}")));
        // 只在角标状态翻转时重建图标，避免频繁分配
        let want = unread > 0;
        if self.badge.replace(want) != want {
            if let Ok(i) = tray_icon::Icon::from_rgba(icon::rgba(32, want), 32, 32) {
                let _ = tray.set_icon(Some(i));
            }
        }
    }

    pub fn quit(&self) {
        if let Some(f) = self.feed.borrow().as_ref() {
            f.stop();
        }
        self.toasts.borrow_mut().close_all();
        self.flush();
        let _ = self.win.hide();
        clear();
        let _ = slint::quit_event_loop();
    }
}
