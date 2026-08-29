mod backend;
mod config;
mod model;
mod popup;
mod store;

use backend::Ev;
use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel, Weak};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::OnceLock;

slint::include_modules!();

static CFG: OnceLock<config::Config> = OnceLock::new();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = CFG.get_or_init(config::Config::load);
    let _ = rustls::crypto::ring::default_provider().install_default();

    let center = Center::new()?;
    let popup = Popup::new()?;
    let tray = TrayArea::new()?;

    // 数据层
    let model = Rc::new(VecModel::<NItem>::default());
    center.set_items(ModelRc::from(model.clone()));
    let store = Rc::new(RefCell::new(store::Store::new(model.clone())));

    let (tx, rx) = mpsc::channel::<Ev>();

    // 通知中心回调
    center.on_open_item({
        let st = store.clone();
        let (wc, wt) = (center.as_weak(), tray.as_weak());
        move |id, url| {
            if !url.is_empty() {
                let u: String = url.into();
                std::thread::spawn(move || {
                    let _ = open::that(&u);
                });
            }
            st.borrow_mut().mark_read(&id);
            refresh(&wc, &wt, &st);
        }
    });
    center.on_del({
        let st = store.clone();
        let (wc, wt) = (center.as_weak(), tray.as_weak());
        move |id| {
            st.borrow_mut().remove(&id);
            refresh(&wc, &wt, &st);
        }
    });
    center.on_clear_all({
        let st = store.clone();
        let (wc, wt) = (center.as_weak(), tray.as_weak());
        move || {
            st.borrow_mut().clear();
            refresh(&wc, &wt, &st);
        }
    });
    center.on_mark_all({
        let st = store.clone();
        let (wc, wt) = (center.as_weak(), tray.as_weak());
        move || {
            st.borrow_mut().mark_all();
            refresh(&wc, &wt, &st);
        }
    });
    center.on_search_changed({
        let st = store.clone();
        let (wc, wt) = (center.as_weak(), tray.as_weak());
        move |s| {
            {
                let mut g = st.borrow_mut();
                g.set_search(s.into());
                g.refresh();
            }
            sync_counts(&wc, &wt, &st);
        }
    });
    center.on_cat_changed({
        let st = store.clone();
        move |c| {
            let mut g = st.borrow_mut();
            g.set_cat(c);
            g.refresh();
        }
    });
    center.window().on_close_requested(|| slint::CloseRequestResponse::HideWindow);

    // 托盘回调
    tray.on_open_center({
        let c = center.as_weak();
        move || {
            if let Some(c) = c.upgrade() {
                let _ = c.show();
            }
        }
    });
    tray.on_mark_all({
        let st = store.clone();
        let (wc, wt) = (center.as_weak(), tray.as_weak());
        move || {
            st.borrow_mut().mark_all();
            refresh(&wc, &wt, &st);
        }
    });
    tray.on_quit(|| {
        let _ = slint::quit_event_loop();
    });

    // 弹窗回调：统一走事件流
    popup.on_dismiss({
        let tx = tx.clone();
        move || {
            let _ = tx.send(Ev::PopDone);
        }
    });
    popup.on_activate({
        let tx = tx.clone();
        let st = store.clone();
        let (wp, wc, wt) = (popup.as_weak(), center.as_weak(), tray.as_weak());
        move || {
            if let Some(p) = wp.upgrade() {
                let cur = p.get_cur();
                if !cur.url.is_empty() {
                    let u: String = cur.url.to_string();
                    std::thread::spawn(move || {
                        let _ = open::that(&u);
                    });
                }
                st.borrow_mut().mark_read(&cur.id);
                refresh(&wc, &wt, &st);
            }
            let _ = tx.send(Ev::PopDone);
        }
    });

    // 后端连接线程
    backend::spawn(cfg.backend.clone(), tx.clone());

    // 事件泵：后端消息/托盘/弹窗时序统一在 UI 线程消费
    let mgr = Rc::new(RefCell::new(popup::PopupMgr::new(
        popup.as_weak(),
        cfg.popup.secs,
        cfg.popup.max_queue,
    )));
    let pump = Timer::default();
    let (wc, wt) = (center.as_weak(), tray.as_weak());
    pump.start(
        TimerMode::Repeated,
        std::time::Duration::from_millis(100),
        move || {
            while let Ok(e) = rx.try_recv() {
                match e {
                    Ev::N(m) => {
                        let it = model::to_item(m);
                        if model::passes(&cfg.filter, &it) {
                            store.borrow_mut().add(it.clone());
                            refresh(&wc, &wt, &store);
                            if cfg.popup.enabled {
                                mgr.borrow_mut().push(it);
                            }
                        }
                    }
                    Ev::St(s) => {
                        if let Some(c) = wc.upgrade() {
                            c.set_conn(s.into());
                        }
                    }
                    Ev::PopDone => mgr.borrow_mut().advance(),
                }
            }
            if mgr.borrow_mut().tick() {
                mgr.borrow_mut().advance();
            }
        },
    );

    if !cfg.ui.start_minimized {
        center.show()?;
    }
    slint::run_event_loop_until_quit()?;
    Ok(())
}

fn sync_counts(w_center: &Weak<Center>, w_tray: &Weak<TrayArea>, st: &Rc<RefCell<store::Store>>) {
    let s = st.borrow();
    if let Some(c) = w_center.upgrade() {
        c.set_unread(s.unread());
        c.set_total(s.total());
    }
    if let Some(t) = w_tray.upgrade() {
        t.set_unread(s.unread());
    }
}

fn refresh(w_center: &Weak<Center>, w_tray: &Weak<TrayArea>, st: &Rc<RefCell<store::Store>>) {
    st.borrow_mut().refresh();
    sync_counts(w_center, w_tray, st);
}
