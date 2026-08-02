use crate::config::Config;
use crate::diagnostics::Diagnostics;
use crate::keypad::{self, KeyGeom, BUTTON_NAMES, KEY_COUNT};
use crate::monitor;
use crate::spiceapi;
use crate::subscreen::{self, SubScreen};
use crate::touchbridge;
use crate::KeyDisc;
use crate::PopnWindow;
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

pub const COMPACT_W: f32 = 132.0;
pub const COMPACT_H: f32 = 60.0;

fn set_if_changed<T: PartialEq>(want: T, current: T, set: impl FnOnce(T)) {
    if want != current {
        set(want);
    }
}

pub struct Runtime {
    pub diag: Rc<RefCell<Diagnostics>>,
    pub bridge: Arc<touchbridge::Bridge>,
    pub sub: Rc<RefCell<SubScreen>>,
    pub rotated_device: Rc<RefCell<Option<String>>>,
    pub api: Rc<RefCell<Option<Rc<spiceapi::Client>>>>,
    pub keys: Rc<VecModel<bool>>,
    pub discs: Rc<VecModel<KeyDisc>>,
    pub log: Rc<VecModel<slint::SharedString>>,
    pub windowed: Rc<Cell<bool>>,
    diag_dirty: Rc<Cell<bool>>,
}

impl Runtime {
    pub fn new(cfg: &Config) -> Self {
        let api = Some(Rc::new(spiceapi::Client::start(cfg.api_port)));
        Self {
            diag: Rc::new(RefCell::new(Diagnostics::new())),
            bridge: touchbridge::Bridge::create(),
            sub: Rc::new(RefCell::new(SubScreen::default())),
            rotated_device: Rc::new(RefCell::new(None)),
            api: Rc::new(RefCell::new(api)),
            keys: Rc::new(VecModel::from(vec![false; KEY_COUNT])),
            discs: Rc::new(VecModel::from(vec![KeyDisc::default(); KEY_COUNT])),
            log: Rc::new(VecModel::default()),
            windowed: Rc::new(Cell::new(cfg.windowed)),
            diag_dirty: Rc::new(Cell::new(true)),
        }
    }

    pub fn apply_config(w: &PopnWindow, cfg: &Config) {
        w.set_playfield_fit(cfg.playfield_fit);
        w.set_button_size(cfg.button_size);
        w.set_panel_pos(cfg.panel_pos);
        w.set_single_row(cfg.single_row);
        w.set_diagnostics(cfg.diagnostics);
        w.set_api_port(cfg.api_port as i32);
        w.set_flip_vertical(cfg.flip_vertical);
        w.set_windowed(cfg.windowed);
        w.set_fit_sub_screen(cfg.fit_sub_screen);
        w.set_pad_monitor(
            monitor::resolve_index(cfg.pad_monitor, monitor::default_pad_index()) as i32,
        );
    }

    pub fn read_config(w: &PopnWindow) -> Config {
        Config {
            playfield_fit: w.get_playfield_fit(),
            button_size: w.get_button_size(),
            panel_pos: w.get_panel_pos(),
            single_row: w.get_single_row(),
            diagnostics: w.get_diagnostics(),
            api_port: w.get_api_port().clamp(1, 65535) as u16,
            flip_vertical: w.get_flip_vertical(),
            windowed: w.get_windowed(),
            pad_monitor: w.get_pad_monitor(),
            fit_sub_screen: w.get_fit_sub_screen(),
        }
    }

    pub fn fill_monitor_options(w: &PopnWindow) {
        let labels = monitor::labels();
        let model: Rc<VecModel<slint::SharedString>> = Rc::new(VecModel::from(
            labels.into_iter().map(Into::into).collect::<Vec<_>>(),
        ));
        w.set_monitor_options(ModelRc::from(model));
    }

    pub fn pad_mon(w: &PopnWindow) -> Option<monitor::MonInfo> {
        monitor::by_index(w.get_pad_monitor().max(0) as usize)
    }

    pub fn compact_rect(w: &PopnWindow, m: &monitor::MonInfo) -> touchbridge::PinRect {
        let scale = w.window().scale_factor().max(0.01);
        touchbridge::PinRect {
            left: m.left,
            top: m.top,
            width: (COMPACT_W * scale).round() as i32,
            height: (COMPACT_H * scale).round() as i32,
        }
    }

    pub fn full_rect(m: &monitor::MonInfo) -> touchbridge::PinRect {
        touchbridge::PinRect {
            left: m.left,
            top: m.top,
            width: m.width.max(1),
            height: m.height.max(1),
        }
    }

    pub fn pad_rect(w: &PopnWindow) -> Option<touchbridge::PinRect> {
        let m = Self::pad_mon(w)?;
        Some(if w.get_nav_active() {
            Self::compact_rect(w, &m)
        } else {
            Self::full_rect(&m)
        })
    }

    pub fn place(w: &PopnWindow, r: touchbridge::PinRect) {
        w.window()
            .set_position(slint::PhysicalPosition::new(r.left, r.top));
        w.window().set_size(slint::PhysicalSize::new(
            r.width.max(1) as u32,
            r.height.max(1) as u32,
        ));
    }

    pub fn pin(w: &PopnWindow) {
        if let Some(r) = Self::pad_rect(w) {
            Self::place(w, r);
        }
    }

    pub fn dock_pad(bridge: &touchbridge::Bridge, w: &PopnWindow, windowed: bool) {
        let nav = w.get_nav_active();
        bridge.set_topmost(nav || !windowed);
        if windowed && !nav {
            bridge.set_pinned(None);
            Self::make_windowed(w);
            return;
        }
        let Some(r) = Self::pad_rect(w) else { return };
        log::info!(
            "pad docked {} at {},{} {}×{}",
            if nav {
                "beside the Sub Screen"
            } else {
                "full screen"
            },
            r.left,
            r.top,
            r.width,
            r.height
        );
        bridge.set_pinned(Some(r));
        Self::place(w, r);
    }

    pub fn make_windowed(w: &PopnWindow) {
        let Some(m) = Self::pad_mon(w) else {
            return;
        };
        let width = 1280.min((m.width - 80).max(900));
        let height = 720.min((m.height - 80).max(600));
        w.window()
            .set_size(slint::PhysicalSize::new(width as u32, height as u32));
        w.window().set_position(slint::PhysicalPosition::new(
            m.left + (m.width - width) / 2,
            m.top + (m.height - height) / 2,
        ));
    }

    pub fn repin_if_needed(w: &PopnWindow) {
        let Some(r) = Self::pad_rect(w) else {
            return;
        };
        let pos = slint::PhysicalPosition::new(r.left, r.top);
        let size = slint::PhysicalSize::new(r.width.max(1) as u32, r.height.max(1) as u32);
        let win = w.window();
        if win.position() != pos || win.size() != size {
            log::info!(
                "pad window drifted to {:?} {:?}; re-pinning to {},{} {}×{}",
                win.position(),
                win.size(),
                r.left,
                r.top,
                r.width,
                r.height
            );
            Self::place(w, r);
        }
    }

    fn sync_sub_screen(sub: &RefCell<SubScreen>, w: &PopnWindow) {
        let mut sub = sub.borrow_mut();
        match (w.get_fit_sub_screen(), Self::pad_mon(w)) {
            (true, Some(m)) => {
                let _ = sub.fit(subscreen::Rect::of(&m));
            }
            _ => sub.restore(),
        }
    }

    pub fn start_pin_watch(&self, w: &PopnWindow) -> slint::Timer {
        let weak = w.as_weak();
        let windowed = self.windowed.clone();
        let bridge = self.bridge.clone();
        let sub = self.sub.clone();
        let t = slint::Timer::default();
        t.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(1500),
            move || {
                let Some(w) = weak.upgrade() else { return };
                Self::sync_sub_screen(&sub, &w);
                if windowed.get() && !w.get_nav_active() {
                    bridge.set_pinned(None);
                    return;
                }
                if bridge.restore_if_minimized() {
                    return;
                }
                bridge.set_pinned(Self::pad_rect(&w));
                Self::repin_if_needed(&w);
            },
        );
        t
    }

    pub fn geom_of(w: &PopnWindow) -> KeyGeom {
        KeyGeom::layout(
            keypad::Band {
                x: 0.0,
                y: 0.0,
                w: w.get_panel_w(),
                h: w.get_panel_h(),
            },
            w.get_playfield_fit(),
            w.get_button_size(),
            w.get_panel_pos(),
            w.get_single_row(),
        )
    }

    fn push_geom(
        bridge: &touchbridge::Bridge,
        discs: &VecModel<KeyDisc>,
        w: &PopnWindow,
        geom: KeyGeom,
    ) {
        bridge.set_key_geom(geom.offset_y(w.get_panel_y()));
        discs.set_vec(
            geom.keys
                .iter()
                .map(|d| KeyDisc {
                    cx: d.cx,
                    cy: d.cy,
                    r: d.r,
                })
                .collect::<Vec<_>>(),
        );
        w.set_band_x(geom.band.x);
        w.set_band_y(geom.band.y);
        w.set_band_w(geom.band.w);
        w.set_band_box_h(geom.band.h);
    }

    pub fn sync_geom(&self, w: &PopnWindow) {
        Self::push_geom(&self.bridge, &self.discs, w, Self::geom_of(w));
    }

    pub fn sync_keys_enabled(bridge: &touchbridge::Bridge, w: &PopnWindow) {
        let want = w.get_keys_interactive();
        if bridge.keys_enabled() != want {
            bridge.set_keys_enabled(want);
        }
    }

    pub fn sync_pad_orientation(w: &PopnWindow, rotated_device: &RefCell<Option<String>>) {
        let flip = w.get_flip_vertical();
        let Some(pad) = Self::pad_mon(w) else {
            return;
        };

        let prev = rotated_device.borrow().clone();
        if let Some(old) = prev {
            if old != pad.device || !flip {
                if let Err(e) = monitor::set_flipped(&old, false) {
                    log::warn!("restore orientation on {old}: {e}");
                }
                *rotated_device.borrow_mut() = None;
            }
        }

        if flip {
            match monitor::set_flipped(&pad.device, true) {
                Ok(()) => *rotated_device.borrow_mut() = Some(pad.device.clone()),
                Err(e) => {
                    log::warn!("flip pad monitor failed: {e}");
                    w.set_flip_vertical(false);
                    w.set_nav_status(format!("Flip failed: {e}").into());
                }
            }
        }
    }

    pub fn bind(&self, w: &PopnWindow) {
        w.set_keys(ModelRc::from(self.keys.clone()));
        w.set_key_discs(ModelRc::from(self.discs.clone()));
        w.set_log(ModelRc::from(self.log.clone()));
        w.set_mouse_keys(true);
        self.bridge.set_topmost(!self.windowed.get());

        self.bind_nav(w);
        self.bind_keys();
        self.bind_mouse_key(w);
        self.bind_mark(w);
        self.bind_quit(w);
        self.bind_settings(w);
    }

    fn bind_mark(&self, w: &PopnWindow) {
        let weak = w.as_weak();
        let diag = self.diag.clone();
        let api = self.api.clone();
        let bridge = self.bridge.clone();
        let keys = self.keys.clone();
        let log_model = self.log.clone();

        w.on_mark_stall(move || {
            let touch = bridge.stats();
            let api_snap = match api.borrow().as_ref() {
                Some(c) => c.debug_snapshot(),
                None => "spiceapi client: none".into(),
            };
            let detail = format!(
                "touch live={} peak={} dropped={} vetoed_moves={} expired={}\n{api_snap}",
                touch.live,
                touch.peak,
                touch.dropped,
                bridge.vetoed_moves(),
                bridge.expired_contacts()
            );
            let banner = diag.borrow_mut().mark_stall(&detail);
            let Some(w) = weak.upgrade() else { return };
            Self::refresh_diagnostics(&w, &diag, &keys, &log_model);
            let _ = banner;
        });
    }

    fn bind_nav(&self, w: &PopnWindow) {
        let weak = w.as_weak();
        let sub = self.sub.clone();
        let bridge = self.bridge.clone();
        let windowed = self.windowed.clone();
        w.on_nav_toggled(move || {
            let Some(w) = weak.upgrade() else { return };
            if w.get_nav_active() {
                Self::sync_sub_screen(&sub, &w);
                if let Err(e) = sub.borrow_mut().raise() {
                    log::warn!("Nav: {e}");
                    w.set_nav_active(false);
                    w.set_nav_status(e.into());
                    return;
                }
                w.set_nav_status("".into());
                bridge.set_keys_enabled(false);
                Self::dock_pad(&bridge, &w, windowed.get());
                log::info!("Nav: Sub Screen to the front; pad docked to its NAV button");
            } else {
                Self::dock_pad(&bridge, &w, windowed.get());
                bridge.set_keys_enabled(w.get_keys_interactive());
                w.set_nav_status("".into());
                log::info!("Nav: pad back to the front");
            }
        });
    }

    fn bind_keys(&self) {
        let diag = self.diag.clone();
        let api = self.api.clone();
        let dirty = self.diag_dirty.clone();

        self.bridge.set_key_listener(move |index, down| {
            let name = BUTTON_NAMES[index.min(BUTTON_NAMES.len() - 1)];
            if let Some(api) = api.borrow().as_ref() {
                api.button(name, down);
            }
            diag.borrow_mut().on_digital(name, down);
            dirty.set(true);
        });
    }

    fn refresh_diagnostics(
        w: &PopnWindow,
        diag: &RefCell<Diagnostics>,
        keys: &VecModel<bool>,
        log: &VecModel<slint::SharedString>,
    ) {
        let d = diag.borrow();
        for (i, on) in d.keys().iter().enumerate() {
            if keys.row_data(i) != Some(*on) {
                keys.set_row_data(i, *on);
            }
        }
        if !w.get_diagnostics() {
            return;
        }
        log.set_vec(
            d.log_lines()
                .map(slint::SharedString::from)
                .collect::<Vec<_>>(),
        );
        w.set_digital_events(d.digital_events() as i32);
    }

    fn bind_mouse_key(&self, w: &PopnWindow) {
        let bridge = self.bridge.clone();
        w.on_mouse_key(move |x, y, up| {
            bridge.track_mouse(x, y, up);
        });
    }

    fn bind_quit(&self, w: &PopnWindow) {
        let rotated_device = self.rotated_device.clone();
        let sub = self.sub.clone();
        w.on_quit(move || {
            sub.borrow_mut().restore();
            if let Some(dev) = rotated_device.borrow_mut().take() {
                let _ = monitor::set_flipped(&dev, false);
            }
            slint::quit_event_loop().unwrap_or_default();
        });
    }

    fn bind_settings(&self, w: &PopnWindow) {
        let weak = w.as_weak();
        let api = self.api.clone();
        let bridge = self.bridge.clone();
        let discs = self.discs.clone();
        let sub = self.sub.clone();
        let rotated_device = self.rotated_device.clone();
        let windowed = self.windowed.clone();

        w.on_settings_changed(move || {
            let Some(w) = weak.upgrade() else { return };
            Self::fill_monitor_options(&w);
            let cfg = Self::read_config(&w);
            cfg.save();

            let want = Some(cfg.api_port);
            let cur = api.borrow().as_ref().map(|c| c.port());
            if cur != want {
                *api.borrow_mut() = want.map(|p| Rc::new(spiceapi::Client::start(p)));
            }

            Self::sync_pad_orientation(&w, &rotated_device);
            Self::fill_monitor_options(&w);
            w.set_pad_monitor(monitor::resolve_index(
                w.get_pad_monitor(),
                monitor::default_pad_index(),
            ) as i32);

            let was_windowed = windowed.replace(cfg.windowed);
            if cfg.windowed && was_windowed && !w.get_nav_active() {
                bridge.set_pinned(None);
                bridge.set_topmost(false);
            } else {
                Self::dock_pad(&bridge, &w, cfg.windowed);
            }

            Self::sync_sub_screen(&sub, &w);

            Self::push_geom(&bridge, &discs, &w, Self::geom_of(&w));
            Self::sync_keys_enabled(&bridge, &w);
        });
    }

    pub fn start_tick(&self, w: &PopnWindow) -> slint::Timer {
        let bridge = self.bridge.clone();
        let weak = w.as_weak();
        let api = self.api.clone();
        let diag = self.diag.clone();
        let keys = self.keys.clone();
        let discs = self.discs.clone();
        let log = self.log.clone();
        let dirty = self.diag_dirty.clone();
        let windowed = self.windowed.clone();
        let installed = std::cell::Cell::new(false);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let last_tick = std::cell::Cell::new(std::time::Instant::now());
        let last_geom = std::cell::Cell::new(KeyGeom::default());
        let last_diag_open = std::cell::Cell::new(false);
        let api_epoch = std::cell::Cell::new(0u64);
        const TICK_LATE: std::time::Duration = std::time::Duration::from_millis(250);
        let t = slint::Timer::default();
        t.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(16),
            move || {
                let late = last_tick.replace(std::time::Instant::now()).elapsed();
                if late > TICK_LATE {
                    log::warn!(
                        "UI thread stalled {:.0} ms — no touch delivered in that window",
                        late.as_secs_f32() * 1000.0
                    );
                }
                let Some(w) = weak.upgrade() else { return };
                if !installed.get() {
                    if touchbridge::install(&bridge, w.as_weak()) {
                        log::info!("touch interception installed");
                        installed.set(true);
                        w.set_mouse_keys(false);
                    } else if std::time::Instant::now() > deadline {
                        log::error!(
                            "touch interception never installed; mouse key fallback active"
                        );
                        installed.set(true);
                        w.set_mouse_keys(true);
                    }
                }

                if bridge.take_display_change() && !windowed.get() {
                    if let Some(r) = Self::pad_rect(&w) {
                        log::info!("re-pinning pad after display change");
                        bridge.set_pinned(Some(r));
                        Self::repin_if_needed(&w);
                    }
                }

                let geom = Self::geom_of(&w);
                if geom != last_geom.get() {
                    last_geom.set(geom);
                    Self::push_geom(&bridge, &discs, &w, geom);
                }
                Self::sync_keys_enabled(&bridge, &w);
                bridge.expire_lost_contacts();

                let diag_open = w.get_diagnostics();
                if dirty.replace(false) || diag_open != last_diag_open.replace(diag_open) {
                    Self::refresh_diagnostics(&w, &diag, &keys, &log);
                }

                let s = bridge.stats();
                set_if_changed(s.live as i32, w.get_live_contacts(), |v| {
                    w.set_live_contacts(v)
                });
                set_if_changed(s.peak as i32, w.get_peak_contacts(), |v| {
                    w.set_peak_contacts(v)
                });
                set_if_changed(s.dropped as i32, w.get_dropped_contacts(), |v| {
                    w.set_dropped_contacts(v)
                });
                match api.borrow().as_ref() {
                    Some(api) => {
                        let epoch = api.status_epoch();
                        if epoch != api_epoch.replace(epoch) {
                            w.set_api_status(api.status().into());
                        }
                        set_if_changed(api.connected(), w.get_api_connected(), |v| {
                            w.set_api_connected(v)
                        });
                    }
                    None => {
                        const DISABLED: u64 = u64::MAX;
                        if api_epoch.replace(DISABLED) != DISABLED {
                            w.set_api_status("disabled".into());
                        }
                    }
                }
            },
        );
        t
    }
}
