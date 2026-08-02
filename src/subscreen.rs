use crate::monitor::MonInfo;

pub const SUB_SCREEN_SUFFIX: &str = "Sub Screen";

const RESCAN: std::time::Duration = std::time::Duration::from_millis(1000);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn of(mon: &MonInfo) -> Self {
        Rect {
            left: mon.left,
            top: mon.top,
            width: mon.width.max(1),
            height: mon.height.max(1),
        }
    }

    pub fn valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

fn missing() -> String {
    format!("no \"…{SUB_SCREEN_SUFFIX}\" window — start the game first")
}

#[cfg(windows)]
#[derive(Default)]
pub struct SubScreen {
    hwnd: isize,
    next_scan: Option<std::time::Instant>,
    announced: isize,
    saved: Option<win::Saved>,
}

#[cfg(windows)]
impl SubScreen {
    pub fn locate(&mut self) -> Option<isize> {
        if self.hwnd != 0 {
            if win::alive(self.hwnd) {
                return Some(self.hwnd);
            }
            log::info!("the \"…{SUB_SCREEN_SUFFIX}\" window went away");
            self.hwnd = 0;
            self.announced = 0;
            self.saved = None;
        }
        let now = std::time::Instant::now();
        if self.next_scan.is_some_and(|due| now < due) {
            return None;
        }
        self.next_scan = Some(now + RESCAN);

        let hwnd = win::find_window_ending_with(SUB_SCREEN_SUFFIX)?;
        self.hwnd = hwnd;
        if self.announced != hwnd {
            self.announced = hwnd;
            log::info!("locked onto the \"…{SUB_SCREEN_SUFFIX}\" window");
        }
        Some(hwnd)
    }

    fn remember(&mut self, hwnd: isize) {
        if self.saved.is_none() {
            self.saved = win::snapshot(hwnd);
        }
    }

    pub fn raise(&mut self) -> Result<(), String> {
        let hwnd = self.locate().ok_or_else(missing)?;
        self.remember(hwnd);
        win::unminimize(hwnd);
        win::raise(hwnd);
        Ok(())
    }

    pub fn fit(&mut self, target: Rect) -> Result<(), String> {
        if !target.valid() {
            return Err("pad monitor has no usable rect".into());
        }
        let hwnd = self.locate().ok_or_else(missing)?;
        if win::is_iconic(hwnd) || win::already_fitted(hwnd, target) {
            return Ok(());
        }
        self.remember(hwnd);
        win::make_borderless(hwnd, target)?;
        log::info!(
            "fitted the Sub Screen to {}×{} at {},{}",
            target.width,
            target.height,
            target.left,
            target.top
        );
        Ok(())
    }

    pub fn restore(&mut self) {
        let Some(saved) = self.saved.take() else {
            return;
        };
        log::info!("restoring the Sub Screen's own frame and placement");
        win::restore(saved);
    }
}

#[cfg(not(windows))]
#[derive(Default)]
pub struct SubScreen;

#[cfg(not(windows))]
impl SubScreen {
    pub fn locate(&mut self) -> Option<isize> {
        None
    }

    pub fn raise(&mut self) -> Result<(), String> {
        Err("the Sub Screen switch is Windows-only".into())
    }

    pub fn fit(&mut self, _target: Rect) -> Result<(), String> {
        Err("the Sub Screen switch is Windows-only".into())
    }

    pub fn restore(&mut self) {}
}

#[cfg(windows)]
mod win {
    use super::Rect;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, GetWindowPlacement, GetWindowRect, IsIconic, IsWindow, IsWindowVisible,
        SetWindowLongPtrW, SetWindowPlacement, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE,
        HWND_NOTOPMOST, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOOWNERZORDER, SWP_NOSIZE, SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED, SW_SHOWMINNOACTIVE,
        SW_SHOWNOACTIVATE, WINDOWPLACEMENT, WS_CAPTION, WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME,
        WS_EX_STATICEDGE, WS_EX_TOPMOST, WS_EX_WINDOWEDGE, WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
        WS_SYSMENU, WS_THICKFRAME,
    };

    const FRAME: u32 = WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU;
    const FRAME_EX: u32 =
        WS_EX_DLGMODALFRAME | WS_EX_CLIENTEDGE | WS_EX_STATICEDGE | WS_EX_WINDOWEDGE;

    const SHOW_MAXIMIZED: u32 = SW_SHOWMAXIMIZED as u32;
    const SHOW_MINIMIZED: u32 = SW_SHOWMINIMIZED as u32;
    const SHOW_MIN_NOACTIVE: u32 = SW_SHOWMINNOACTIVE as u32;
    const SHOW_NOACTIVATE: u32 = SW_SHOWNOACTIVATE as u32;

    #[derive(Clone, Copy)]
    pub struct Saved {
        hwnd: isize,
        style: u32,
        ex_style: u32,
        placement: WINDOWPLACEMENT,
    }

    pub fn alive(hwnd: isize) -> bool {
        let h = hwnd as HWND;
        unsafe { IsWindow(h) != 0 && IsWindowVisible(h) != 0 }
    }

    pub fn is_iconic(hwnd: isize) -> bool {
        unsafe { IsIconic(hwnd as HWND) != 0 }
    }

    pub fn unminimize(hwnd: isize) {
        if is_iconic(hwnd) {
            unsafe { ShowWindow(hwnd as HWND, SW_SHOWNOACTIVATE) };
        }
    }

    pub fn raise(hwnd: isize) {
        unsafe {
            SetWindowPos(
                hwnd as HWND,
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            );
        }
    }

    pub fn window_rect(hwnd: isize) -> Option<Rect> {
        unsafe {
            let mut rc: RECT = core::mem::zeroed();
            if GetWindowRect(hwnd as HWND, &mut rc) == 0 {
                return None;
            }
            Some(Rect {
                left: rc.left,
                top: rc.top,
                width: rc.right - rc.left,
                height: rc.bottom - rc.top,
            })
        }
    }

    fn styles(hwnd: isize) -> (u32, u32) {
        unsafe {
            (
                GetWindowLongPtrW(hwnd as HWND, GWL_STYLE) as u32,
                GetWindowLongPtrW(hwnd as HWND, GWL_EXSTYLE) as u32,
            )
        }
    }

    pub fn already_fitted(hwnd: isize, target: Rect) -> bool {
        styles(hwnd).0 & FRAME == 0 && window_rect(hwnd) == Some(target)
    }

    pub fn snapshot(hwnd: isize) -> Option<Saved> {
        let (style, ex_style) = styles(hwnd);
        unsafe {
            let mut placement: WINDOWPLACEMENT = core::mem::zeroed();
            placement.length = core::mem::size_of::<WINDOWPLACEMENT>() as u32;
            if GetWindowPlacement(hwnd as HWND, &mut placement) == 0 {
                log::warn!("could not read the Sub Screen's placement; leaving it alone");
                return None;
            }
            Some(Saved {
                hwnd,
                style,
                ex_style,
                placement,
            })
        }
    }

    pub fn make_borderless(hwnd: isize, target: Rect) -> Result<(), String> {
        let h = hwnd as HWND;
        let (style, ex_style) = styles(hwnd);
        unsafe {
            ShowWindow(h, SW_SHOWNOACTIVATE);
            SetWindowLongPtrW(h, GWL_STYLE, (style & !FRAME) as isize);
            SetWindowLongPtrW(h, GWL_EXSTYLE, (ex_style & !FRAME_EX) as isize);
            let ok = SetWindowPos(
                h,
                HWND_NOTOPMOST,
                target.left,
                target.top,
                target.width,
                target.height,
                SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            );
            if ok == 0 {
                return Err("SetWindowPos on the Sub Screen failed".into());
            }
        }
        Ok(())
    }

    pub fn restore(saved: Saved) {
        let h = saved.hwnd as HWND;
        if unsafe { IsWindow(h) } == 0 {
            return;
        }
        let mut placement = saved.placement;
        placement.showCmd = match placement.showCmd {
            SHOW_MAXIMIZED => SHOW_MAXIMIZED,
            SHOW_MINIMIZED | SHOW_MIN_NOACTIVE => SHOW_MIN_NOACTIVE,
            _ => SHOW_NOACTIVATE,
        };
        let band = if saved.ex_style & WS_EX_TOPMOST != 0 {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        unsafe {
            SetWindowLongPtrW(h, GWL_STYLE, saved.style as isize);
            SetWindowLongPtrW(h, GWL_EXSTYLE, saved.ex_style as isize);
            SetWindowPos(
                h,
                band,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            );
            SetWindowPlacement(h, &placement);
        }
    }

    struct FindCtx {
        suffix: String,
        found: HWND,
    }

    pub fn find_window_ending_with(suffix: &str) -> Option<isize> {
        use windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows;
        let mut ctx = FindCtx {
            suffix: suffix.to_string(),
            found: std::ptr::null_mut(),
        };
        unsafe {
            EnumWindows(Some(enum_cb), &mut ctx as *mut FindCtx as LPARAM);
        }
        (!ctx.found.is_null()).then_some(ctx.found as isize)
    }

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> i32 {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowTextW, IsWindowVisible};
        let ctx = &mut *(lparam as *mut FindCtx);
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let mut buf = [0u16; 256];
        let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if n <= 0 {
            return 1;
        }
        let title = String::from_utf16_lossy(&buf[..n as usize]);
        if title.ends_with(&ctx.suffix) {
            ctx.found = hwnd;
            return 0;
        }
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(index: usize) -> MonInfo {
        MonInfo {
            index,
            left: index as i32 * 100,
            top: 0,
            width: 100,
            height: 100,
            primary: index == 0,
            device: format!("display-{index}"),
            label: format!("Display {index}"),
        }
    }

    #[test]
    fn a_monitor_becomes_the_fit_target() {
        assert_eq!(
            Rect::of(&monitor(2)),
            Rect {
                left: 200,
                top: 0,
                width: 100,
                height: 100
            }
        );
    }

    #[test]
    fn a_zero_sized_rect_is_not_a_fit_target() {
        assert!(!Rect::default().valid());
        assert!(Rect::of(&monitor(0)).valid());
    }

    #[test]
    fn the_missing_window_message_names_the_suffix_it_looked_for() {
        assert!(missing().contains(SUB_SCREEN_SUFFIX));
    }

    #[test]
    fn a_target_with_no_size_is_refused_before_any_window_is_touched() {
        assert!(SubScreen::default().fit(Rect::default()).is_err());
    }

    #[cfg(windows)]
    mod windows_only {
        use super::*;
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, GetWindowLongPtrW, GWL_EXSTYLE, GWL_STYLE, WS_CAPTION,
            WS_EX_TOPMOST, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        };

        const OFFSCREEN: i32 = -20000;

        struct StandIn(HWND);

        impl StandIn {
            fn new(title: &str) -> Self {
                Self::with_ex_style(title, 0)
            }

            fn with_ex_style(title: &str, ex_style: u32) -> Self {
                let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
                let mut wide: Vec<u16> = title.encode_utf16().collect();
                wide.push(0);
                let hwnd = unsafe {
                    CreateWindowExW(
                        ex_style,
                        class.as_ptr(),
                        wide.as_ptr(),
                        WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                        OFFSCREEN,
                        OFFSCREEN,
                        320,
                        240,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null(),
                    )
                };
                assert!(!hwnd.is_null(), "could not create the stand-in window");
                StandIn(hwnd)
            }

            fn handle(&self) -> isize {
                self.0 as isize
            }

            fn framed(&self) -> bool {
                unsafe { GetWindowLongPtrW(self.0, GWL_STYLE) as u32 & WS_CAPTION != 0 }
            }

            fn topmost(&self) -> bool {
                unsafe { GetWindowLongPtrW(self.0, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0 }
            }

            fn size(&self) -> (i32, i32) {
                let r = win::window_rect(self.handle()).expect("a live window has a rect");
                (r.width, r.height)
            }
        }

        impl Drop for StandIn {
            fn drop(&mut self) {
                unsafe { DestroyWindow(self.0) };
            }
        }

        fn target() -> Rect {
            Rect {
                left: OFFSCREEN,
                top: OFFSCREEN,
                width: 640,
                height: 480,
            }
        }

        #[test]
        fn the_suffix_sweep_sees_a_sub_screen_window_and_does_not_invent_one() {
            let _stand_in = StandIn::new("popnPad selftest Sub Screen");
            assert!(
                win::find_window_ending_with(SUB_SCREEN_SUFFIX).is_some(),
                "a \"…Sub Screen\" window must be found by suffix, not exact title"
            );
            assert_eq!(
                win::find_window_ending_with("no window is called this at all"),
                None
            );
        }

        #[test]
        fn fitting_strips_the_frame_and_restoring_puts_it_back() {
            let stand_in = StandIn::new("popnPad fit selftest Sub Screen");
            assert!(stand_in.framed(), "the stand-in starts framed");
            assert_eq!(stand_in.size(), (320, 240));

            let saved = win::snapshot(stand_in.handle()).expect("placement snapshot");
            win::make_borderless(stand_in.handle(), target()).expect("fit");

            assert!(!stand_in.framed(), "fitting takes the caption off");
            assert_eq!(
                win::window_rect(stand_in.handle()),
                Some(target()),
                "and sizes the window to the target rect"
            );
            assert!(
                win::already_fitted(stand_in.handle(), target()),
                "so a second fit is a cheap no-op rather than a reshape"
            );

            win::restore(saved);
            assert!(stand_in.framed(), "restoring gives the frame back");
            assert_eq!(
                stand_in.size(),
                (320, 240),
                "and the window its original size"
            );
        }

        #[test]
        fn a_topmost_sub_screen_is_demoted_so_the_pad_dock_stays_reachable() {
            let stand_in =
                StandIn::with_ex_style("popnPad topmost selftest Sub Screen", WS_EX_TOPMOST);
            assert!(stand_in.topmost(), "the stand-in starts topmost");

            let saved = win::snapshot(stand_in.handle()).expect("placement snapshot");
            win::raise(stand_in.handle());
            assert!(
                !stand_in.topmost(),
                "a topmost Sub Screen would sit above the pad's dock and strand the user"
            );

            win::restore(saved);
            assert!(
                stand_in.topmost(),
                "but its own z-band is given back on restore"
            );
        }

        #[test]
        fn an_unfitted_window_never_looks_already_fitted() {
            let stand_in = StandIn::new("popnPad unfitted selftest Sub Screen");
            assert!(!win::already_fitted(stand_in.handle(), target()));
        }

        #[test]
        fn restore_with_nothing_saved_is_a_no_op() {
            let stand_in = StandIn::new("popnPad idle selftest Sub Screen");
            let mut sub = SubScreen::default();
            sub.restore();
            sub.restore();
            assert!(stand_in.framed(), "an untouched window is left alone");
        }

        #[test]
        fn a_destroyed_window_stops_reading_as_a_live_sub_screen() {
            let handle = {
                let stand_in = StandIn::new("popnPad transient selftest Sub Screen");
                let handle = stand_in.handle();
                assert!(win::alive(handle));
                handle
            };
            assert!(
                !win::alive(handle),
                "a dead HWND must not keep the switch pointed at stale coordinates"
            );
        }

        #[test]
        fn restoring_a_destroyed_window_does_not_blow_up() {
            let stand_in = StandIn::new("popnPad doomed selftest Sub Screen");
            let saved = win::snapshot(stand_in.handle()).expect("placement snapshot");
            drop(stand_in);
            win::restore(saved);
        }
    }
}
