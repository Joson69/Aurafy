//! Desktop integration. Tray resources live on eframe's Windows event-loop thread.
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WindowPlacement {
    pub normal_rect: [i32; 4],
    pub maximized: bool,
}

impl WindowPlacement {
    fn valid(&self) -> bool {
        let [left, top, right, bottom] = self.normal_rect.map(i64::from);
        (200..=32_000).contains(&(right - left))
            && (150..=32_000).contains(&(bottom - top))
            && self
                .normal_rect
                .iter()
                .all(|v| v.unsigned_abs() < 1_000_000)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Open,
    Play,
    Pause,
    TogglePlayback,
    Next,
    Previous,
    Quit,
}

pub fn window_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/aurafy.png"))
        .expect("Embedded Aurafy icon must be a valid PNG")
        .into_rgba8();
    egui::IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

pub fn logo_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let image = image::load_from_memory(include_bytes!("../assets/aurafy-header.png"))
        .expect("Embedded Aurafy logo must be a valid PNG")
        .into_rgba8();
    ctx.load_texture(
        "aurafy-logo",
        egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    )
}

#[derive(Default)]
pub struct WindowState {
    pub hidden: bool,
    pub quitting: bool,
}

impl WindowState {
    pub fn close(&mut self, tray_available: bool, quit_on_close: bool, ctx: &egui::Context) {
        if tray_available && !quit_on_close && !self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.hidden = true;
        }
    }

    pub fn open(&mut self, ctx: &egui::Context) {
        self.hidden = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    pub fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

#[cfg(target_os = "windows")]
pub use windows::{Geometry, Instance, Tray};

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use crate::AppEvent;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::{
        io,
        os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc::Sender,
        },
        thread,
    };
    use tray_icon::{
        MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
        menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    };
    use windows_sys::Win32::{
        Foundation::{ERROR_ALREADY_EXISTS, GetLastError, RECT, WAIT_OBJECT_0},
        Graphics::Gdi::{
            GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
            RDW_INVALIDATE, RedrawWindow,
        },
        System::Threading::{CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject},
        UI::WindowsAndMessaging::{
            GetClientRect, GetWindowPlacement, IsIconic, IsWindowVisible, KillTimer, PostMessageW,
            SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED, SW_SHOWNORMAL, SetTimer, SetWindowPlacement,
            WINDOWPLACEMENT, WM_PAINT, WPF_RESTORETOMAXIMIZED,
        },
    };

    #[derive(Clone, Copy)]
    pub struct Geometry(isize);

    impl Geometry {
        pub fn wake_repaint(self, ctx: &egui::Context) {
            ctx.request_repaint();
            // SAFETY: this HWND belongs to eframe and outlives background playback threads.
            unsafe {
                PostMessageW(self.0 as _, WM_PAINT, 0, 0);
            }
        }

        pub fn visible(&self) -> bool {
            // SAFETY: the owning App keeps this HWND alive.
            unsafe { IsWindowVisible(self.0 as _) != 0 }
        }

        pub fn display_bounds(&self) -> Option<[i32; 8]> {
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            // The HWND belongs to eframe and the monitor lookup runs on its UI thread.
            let monitor = unsafe { MonitorFromWindow(self.0 as _, MONITOR_DEFAULTTONEAREST) };
            if monitor.is_null() || unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
                return None;
            }
            let screen = info.rcMonitor;
            let work = info.rcWork;
            Some([
                screen.left,
                screen.top,
                screen.right,
                screen.bottom,
                work.left,
                work.top,
                work.right,
                work.bottom,
            ])
        }

        pub fn collapsed(&self) -> bool {
            let mut rect = RECT::default();
            unsafe {
                self.visible()
                    && IsIconic(self.0 as _) == 0
                    && GetClientRect(self.0 as _, &mut rect) != 0
                    && (rect.right - rect.left < 200 || rect.bottom - rect.top < 150)
            }
        }

        pub fn invalidate(&self) {
            // Queue a full client repaint without recursing into eframe's update.
            unsafe {
                RedrawWindow(
                    self.0 as _,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    RDW_INVALIDATE,
                );
            }
        }
        pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
            Wake::new(cc).map(|window| Self(window.0))
        }

        pub fn capture(&self) -> Option<WindowPlacement> {
            let mut native = WINDOWPLACEMENT {
                length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
                ..Default::default()
            };
            // SAFETY: the HWND belongs to eframe and remains valid while the App exists.
            if unsafe { GetWindowPlacement(self.0 as _, &mut native) } == 0 {
                return None;
            }
            let rect = native.rcNormalPosition;
            let saved = WindowPlacement {
                normal_rect: [rect.left, rect.top, rect.right, rect.bottom],
                maximized: native.showCmd == SW_SHOWMAXIMIZED as u32
                    || (native.showCmd == SW_SHOWMINIMIZED as u32
                        && native.flags & WPF_RESTORETOMAXIMIZED != 0),
            };
            saved.valid().then_some(saved)
        }

        pub fn restore(&self, saved: WindowPlacement) -> bool {
            if !saved.valid() {
                return false;
            }
            let [left, top, right, bottom] = saved.normal_rect;
            let native = WINDOWPLACEMENT {
                length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
                showCmd: if saved.maximized {
                    SW_SHOWMAXIMIZED
                } else {
                    SW_SHOWNORMAL
                } as u32,
                ptMinPosition: windows_sys::Win32::Foundation::POINT { x: -1, y: -1 },
                ptMaxPosition: windows_sys::Win32::Foundation::POINT { x: -1, y: -1 },
                rcNormalPosition: windows_sys::Win32::Foundation::RECT {
                    left,
                    top,
                    right,
                    bottom,
                },
                ..Default::default()
            };
            // Keep workspace coordinates intact; Windows recovers fully offscreen positions.
            unsafe { SetWindowPlacement(self.0 as _, &native) != 0 }
        }
    }

    #[derive(Clone, Copy)]
    struct Wake(isize);

    impl Wake {
        fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
            match cc.window_handle().map_err(|e| e.to_string())?.as_raw() {
                RawWindowHandle::Win32(handle) => Ok(Self(handle.hwnd.get())),
                _ => Err("Aurafy did not receive a Windows window handle.".into()),
            }
        }

        fn repaint(self, ctx: &egui::Context) {
            ctx.request_repaint();
            // Hidden windows do not get WM_PAINT from winit's RedrawWindow.
            // Post an explicit paint so eframe processes queued actions while hidden.
            // SAFETY: this handle belongs to eframe and outlives our tray/listener.
            unsafe {
                PostMessageW(self.0 as _, WM_PAINT, 0, 0);
            }
        }
    }

    unsafe extern "system" fn hidden_tick(
        hwnd: windows_sys::Win32::Foundation::HWND,
        _: u32,
        _: usize,
        _: u32,
    ) {
        // Process background state/bookmarks once per second without drawing the UI.
        // SAFETY: Windows invokes this timer only while its owning window exists.
        unsafe {
            if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
                PostMessageW(hwnd, WM_PAINT, 0, 0);
            }
        }
    }

    pub struct Tray {
        icon: TrayIcon,
        toggle: MenuItem,
        last_label: String,
        last_playing: Option<bool>,
        window: Wake,
        timer: usize,
    }

    impl Tray {
        pub fn new(
            cc: &eframe::CreationContext<'_>,
            tx: Sender<AppEvent>,
            guid: u128,
            style: crate::appearance::TrayStyle,
        ) -> Result<Self, String> {
            let ctx = &cc.egui_ctx;
            let window = Wake::new(cc)?;
            let menu = Menu::new();
            let open = MenuItem::with_id("open", "Open Aurafy", true, None);
            let toggle = MenuItem::with_id("toggle", "Play", true, None);
            let previous = MenuItem::with_id("previous", "Previous", true, None);
            let next = MenuItem::with_id("next", "Next", true, None);
            let quit = MenuItem::with_id("quit", "Quit Aurafy", true, None);
            menu.append_items(&[
                &open,
                &PredefinedMenuItem::separator(),
                &toggle,
                &previous,
                &next,
                &PredefinedMenuItem::separator(),
                &quit,
            ])
            .map_err(|e| e.to_string())?;
            let icon = tray_icon::Icon::from_rgba(crate::appearance::tray_rgba(style)?, 32, 32)
                .map_err(|e| e.to_string())?;
            let icon = TrayIconBuilder::new()
                // Keep the shell identity/GUID so Windows retains tray preferences.
                .with_id("auralis")
                // A stable identity lets Windows retain the user's tray visibility preference.
                .with_guid(guid)
                .with_icon(icon)
                .with_tooltip("Aurafy")
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()
                .map_err(|e| e.to_string())?;
            // The timer is delivered on eframe's event-loop thread, even while hidden.
            let timer = 0xA083;
            if unsafe { SetTimer(window.0 as _, timer, 1000, Some(hidden_tick)) } == 0 {
                return Err(io::Error::last_os_error().to_string());
            }

            let menu_ctx = ctx.clone();
            let menu_tx = tx.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                let action = match event.id.as_ref() {
                    "open" => Action::Open,
                    "toggle" => Action::TogglePlayback,
                    "previous" => Action::Previous,
                    "next" => Action::Next,
                    "quit" => Action::Quit,
                    _ => return,
                };
                let _ = menu_tx.send(AppEvent::Desktop(action));
                window.repaint(&menu_ctx);
            }));
            let click_ctx = ctx.clone();
            TrayIconEvent::set_event_handler(Some(move |event| {
                if matches!(
                    event,
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                ) {
                    let _ = tx.send(AppEvent::Desktop(Action::Open));
                    window.repaint(&click_ctx);
                }
            }));
            Ok(Self {
                icon,
                toggle,
                last_label: String::new(),
                last_playing: None,
                window,
                timer,
            })
        }

        pub fn update(&mut self, playing: bool, song: &str, artist: &str) {
            if self.last_playing != Some(playing) {
                self.toggle.set_text(if playing { "Pause" } else { "Play" });
                self.last_playing = Some(playing);
            }
            let label: String = format!(
                "Aurafy · {}\n{song} — {artist}",
                if playing { "Playing" } else { "Paused" }
            )
            .chars()
            .take(120)
            .collect();
            if label != self.last_label {
                let _ = self.icon.set_tooltip(Some(&label));
                self.last_label = label;
            }
        }

        pub fn set_style(&self, style: crate::appearance::TrayStyle) -> Result<(), String> {
            let icon = tray_icon::Icon::from_rgba(crate::appearance::tray_rgba(style)?, 32, 32)
                .map_err(|e| e.to_string())?;
            self.icon.set_icon(Some(icon)).map_err(|e| e.to_string())
        }

        pub fn registered(&self) -> bool {
            self.icon.rect().is_some()
        }
    }

    impl Drop for Tray {
        fn drop(&mut self) {
            // SAFETY: this timer was created on our window and must stop before teardown.
            unsafe {
                KillTimer(self.window.0 as _, self.timer);
            }
            MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
            TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
        }
    }

    /// Keep one player instance per Windows login, and reopen it on a second launch.
    /// Named synchronization objects avoid window-title matching or disk polling.
    pub struct Instance {
        _mutex: OwnedHandle,
        event: Arc<OwnedHandle>,
        stop: Arc<AtomicBool>,
        listener: Option<thread::JoinHandle<()>>,
    }

    impl Instance {
        pub fn claim() -> io::Result<Option<Self>> {
            // Share the mutex with earlier releases during the Aurafy upgrade.
            Self::claim_named("Local\\Auralis.Desktop.e8f27dc0")
        }

        fn claim_named(name: &str) -> io::Result<Option<Self>> {
            let event_name: Vec<u16> = format!("{name}.Open\0").encode_utf16().collect();
            let mutex_name: Vec<u16> = format!("{name}.Instance\0").encode_utf16().collect();
            // SAFETY: names are NUL terminated; non-inheritable handles use default ACLs.
            let event = unsafe { CreateEventW(std::ptr::null(), 0, 0, event_name.as_ptr()) };
            if event.is_null() {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: each successful creation returns a newly owned handle.
            let event = Arc::new(unsafe { OwnedHandle::from_raw_handle(event) });
            let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr()) };
            let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
            if mutex.is_null() {
                return Err(io::Error::last_os_error());
            }
            let mutex = unsafe { OwnedHandle::from_raw_handle(mutex) };
            if already_exists {
                // SAFETY: the Arc keeps this event handle alive throughout the call.
                if unsafe { SetEvent(event.as_raw_handle()) } == 0 {
                    return Err(io::Error::last_os_error());
                }
                return Ok(None);
            }
            Ok(Some(Self {
                _mutex: mutex,
                event,
                stop: Arc::new(AtomicBool::new(false)),
                listener: None,
            }))
        }

        pub fn listen(&mut self, cc: &eframe::CreationContext<'_>, tx: Sender<AppEvent>) {
            let ctx = cc.egui_ctx.clone();
            let window = Wake::new(cc).expect("eframe must provide a Windows window");
            let event = self.event.clone();
            let stop = self.stop.clone();
            self.listener = Some(thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    // SAFETY: this worker owns a reference to the event until it exits.
                    let result = unsafe { WaitForSingleObject(event.as_raw_handle(), u32::MAX) };
                    if stop.load(Ordering::Relaxed) || result != WAIT_OBJECT_0 {
                        break;
                    }
                    if tx.send(AppEvent::Desktop(Action::Open)).is_err() {
                        break;
                    }
                    window.repaint(&ctx);
                }
            }));
        }
    }

    impl Drop for Instance {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            // SAFETY: this wakes our listener before joining it and closing its handle.
            unsafe {
                SetEvent(self.event.as_raw_handle());
            }
            if let Some(listener) = self.listener.take() {
                let _ = listener.join();
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn native_window_placement_survives_recreation_and_maximizing() {
            use windows_sys::{
                Win32::UI::WindowsAndMessaging::{
                    CreateWindowExW, DestroyWindow, WS_OVERLAPPEDWINDOW,
                },
                w,
            };
            struct TestWindow(windows_sys::Win32::Foundation::HWND);
            impl TestWindow {
                fn new() -> Self {
                    let handle = unsafe {
                        CreateWindowExW(
                            0,
                            w!("STATIC"),
                            w!("Aurafy geometry check"),
                            WS_OVERLAPPEDWINDOW,
                            0,
                            0,
                            500,
                            350,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            std::ptr::null(),
                        )
                    };
                    assert!(!handle.is_null());
                    Self(handle)
                }
                fn geometry(&self) -> Geometry {
                    Geometry(self.0 as isize)
                }
            }
            impl Drop for TestWindow {
                fn drop(&mut self) {
                    unsafe {
                        DestroyWindow(self.0);
                    }
                }
            }
            let original = WindowPlacement {
                normal_rect: [90, 110, 790, 610],
                maximized: false,
            };
            let first = TestWindow::new();
            assert!(first.geometry().restore(original));
            assert_eq!(first.geometry().capture().unwrap(), original);
            let maximized = WindowPlacement {
                maximized: true,
                ..original
            };
            assert!(first.geometry().restore(maximized));
            assert_eq!(first.geometry().capture().unwrap(), maximized);
            let json = serde_json::to_string(&first.geometry().capture()).unwrap();
            drop(first);
            let second = TestWindow::new();
            let restored: Option<WindowPlacement> = serde_json::from_str(&json).unwrap();
            assert!(second.geometry().restore(restored.unwrap()));
            assert_eq!(second.geometry().capture().unwrap(), maximized);
            assert!(second.geometry().restore(original));
            assert_eq!(second.geometry().capture().unwrap(), original);
            assert!(!second.geometry().restore(WindowPlacement {
                normal_rect: [0, 0, -10, 0],
                maximized: false
            }));
        }

        #[test]
        fn second_launch_notifies_owner_and_guard_releases_on_exit() {
            let name = format!(
                "Local\\Aurafy.Test.{}.{}",
                std::process::id(),
                rand::random::<u64>()
            );
            let first = Instance::claim_named(&name).unwrap().unwrap();
            assert!(Instance::claim_named(&name).unwrap().is_none());
            assert_eq!(
                unsafe { WaitForSingleObject(first.event.as_raw_handle(), 1000) },
                WAIT_OBJECT_0
            );
            drop(first);
            assert!(Instance::claim_named(&name).unwrap().is_some());
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub struct Tray;

#[cfg(not(target_os = "windows"))]
impl Tray {
    pub fn new(
        _: &eframe::CreationContext<'_>,
        _: std::sync::mpsc::Sender<crate::AppEvent>,
        _: u128,
    ) -> Result<Self, String> {
        Err("Tray integration is available on Windows.".into())
    }
    pub fn update(&mut self, _: bool, _: &str, _: &str) {}
    pub fn registered(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closing_hides_only_with_a_working_tray_and_quit_always_exits() {
        let ctx = egui::Context::default();
        for (available, quit_on_close, quitting, hide) in [
            (true, false, false, true),
            (false, false, false, false),
            (true, true, false, false),
            (true, false, true, false),
        ] {
            let mut state = WindowState {
                hidden: false,
                quitting,
            };
            let output = ctx.run(Default::default(), |ctx| {
                state.close(available, quit_on_close, ctx)
            });
            assert_eq!(state.hidden, hide);
            assert_eq!(
                output.viewport_output[&egui::ViewportId::ROOT]
                    .commands
                    .contains(&egui::ViewportCommand::CancelClose),
                hide
            );
        }
        let mut state = WindowState {
            hidden: true,
            quitting: false,
        };
        let output = ctx.run(Default::default(), |ctx| state.open(ctx));
        assert!(!state.hidden);
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::Visible(true))
        );
        let output = ctx.run(Default::default(), |ctx| state.quit(ctx));
        assert!(state.quitting);
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::Close)
        );
    }
}
