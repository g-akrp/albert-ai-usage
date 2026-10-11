#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod draw;
mod platform;
mod settings;

use ai_usage_core::{
    card_model::{self, Card, Run},
    card_order,
    config::ProviderConfig,
    panel_layout::{self, Action, Layout, Rect},
    relative_time,
    report::{Count, Meter, Report, Window},
    runner::{self, Cancellation},
    schedule::Schedule,
    settings_model::Settings,
    store,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::c_void,
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender},
    thread::JoinHandle,
    time::Instant,
};
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::*,
        Graphics::{Dwm::*, Gdi::*},
        System::{Com::*, LibraryLoader::GetModuleHandleW, Threading::*},
        UI::{HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

const UPDATE: u32 = WM_APP + 1;
const TRAY: u32 = WM_APP + 2;
struct Completion {
    id: String,
    interval: u64,
    runs: Vec<Run>,
}
struct App {
    hwnd: HWND,
    tray: Option<platform::Tray>,
    painter: draw::Painter,
    configs: Vec<ProviderConfig>,
    runs: Vec<Run>,
    errors: Vec<String>,
    settings: Settings,
    folder: PathBuf,
    schedule: Schedule,
    tokens: HashMap<String, Cancellation>,
    workers: Vec<JoinHandle<()>>,
    tx: SyncSender<Completion>,
    rx: Receiver<Completion>,
    created: Instant,
    layout: Layout,
    visible: bool,
    scale: f32,
    scroll: f32,
    hover: Option<String>,
    hover_hit: Option<usize>,
    focus: Option<usize>,
    updated: String,
    demo: bool,
    taskbar_message: u32,
}
impl App {
    fn new(demo: bool) -> windows::core::Result<Self> {
        let folder = settings::folder(demo);
        let providers_folder = folder.join("providers");
        let (configs, errors) = store::load(if demo { None } else { Some(&providers_folder) });
        let mut settings = settings::load(&folder);
        if demo && settings.collapsed.is_empty() {
            settings.collapsed.push("codex".into());
        }
        let runs = configs
            .iter()
            .map(|c| if demo { demo_run(c) } else { Run::loading(c) })
            .collect();
        let (tx, rx) = mpsc::sync_channel(2);
        Ok(Self {
            hwnd: HWND::default(),
            tray: None,
            painter: draw::Painter::new()?,
            configs,
            runs,
            errors,
            settings,
            folder,
            schedule: Schedule::default(),
            tokens: HashMap::new(),
            workers: Vec::new(),
            tx,
            rx,
            created: Instant::now(),
            layout: Layout::default(),
            visible: false,
            scale: 1.0,
            scroll: 0.0,
            hover: None,
            hover_hit: None,
            focus: None,
            updated: platform::format_time(relative_time::now()),
            demo,
            taskbar_message: 0,
        })
    }
    fn active_runs(&mut self) -> Vec<Run> {
        let available: Vec<_> = self
            .runs
            .iter()
            .filter(|r| !self.settings.disabled.contains(&r.config_id))
            .map(|r| r.id.clone())
            .collect();
        self.settings.order = card_order::reconcile(&self.settings.order, &available);
        let runs = self
            .settings
            .order
            .iter()
            .filter_map(|id| self.runs.iter().find(|r| &r.id == id))
            .cloned()
            .collect::<Vec<_>>();
        card_model::ensure_pin(&mut self.settings, &runs);
        runs
    }
    fn icon(&mut self, add: bool) {
        let runs = self.active_runs();
        if let Some(tray) = &mut self.tray {
            let _ = tray.update(&runs, self.settings.pin.as_deref(), self.scale, add);
        }
    }
    fn save(&self) {
        let _ = settings::save(&self.folder, &self.settings);
    }
    fn rebuild(&mut self, max_width: f32) {
        self.hover_hit = None;
        let runs = self.active_runs();
        let cards = runs
            .iter()
            .filter(|r| !self.settings.hidden.contains(&r.id))
            .map(Card::from_run)
            .collect::<Vec<_>>();
        self.layout = panel_layout::layout(
            &cards,
            &self.settings,
            &self.errors,
            &self.updated,
            env!("CARGO_PKG_VERSION"),
            !self.demo && platform::login_enabled(),
            !self.settings.overflow_hint_dismissed,
            max_width,
            |s, size| self.painter.measure(s, size),
            platform::format_date,
        );
        self.scroll = self.scroll.min(self.max_scroll());
    }
    fn work_area(&self, anchor: Rect) -> RECT {
        unsafe {
            let monitor = MonitorFromPoint(
                POINT {
                    x: anchor.x as i32,
                    y: anchor.y as i32,
                },
                MONITOR_DEFAULTTONEAREST,
            );
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                info.rcWork
            } else {
                RECT {
                    left: 0,
                    top: 0,
                    right: GetSystemMetrics(SM_CXSCREEN),
                    bottom: GetSystemMetrics(SM_CYSCREEN),
                }
            }
        }
    }
    fn show(&mut self) {
        unsafe {
            let anchor = self
                .tray
                .as_ref()
                .map(platform::Tray::anchor)
                .unwrap_or_default();
            let work = self.work_area(anchor);
            for _ in 0..2 {
                self.rebuild((work.right - work.left) as f32 / self.scale - 16.0);
                let pos = panel_layout::position(
                    anchor,
                    Rect {
                        x: work.left as f32,
                        y: work.top as f32,
                        w: (work.right - work.left) as f32,
                        h: (work.bottom - work.top) as f32,
                    },
                    self.layout.width * self.scale,
                    self.layout.height * self.scale,
                );
                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOPMOST),
                    pos.x as i32,
                    pos.y as i32,
                    pos.w.ceil() as i32,
                    pos.h.ceil() as i32,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                let scale = GetDpiForWindow(self.hwnd) as f32 / 96.0;
                if scale > 0.0 && (scale - self.scale).abs() > 0.01 {
                    self.scale = scale;
                    self.painter.target = None;
                } else {
                    break;
                }
            }
            self.visible = true;
            let _ = SetForegroundWindow(self.hwnd);
            let _ = SetFocus(Some(self.hwnd));
            self.repaint();
        }
    }
    fn hide(&mut self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        self.visible = false;
        self.hover = None;
        self.hover_hit = None;
        self.focus = None;
        self.painter.target = None;
    }
    fn repaint(&self) {
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }
    fn client_size(&self) -> (u32, u32) {
        let mut r = RECT::default();
        unsafe {
            let _ = GetClientRect(self.hwnd, &mut r);
        }
        (
            (r.right - r.left).max(1) as u32,
            (r.bottom - r.top).max(1) as u32,
        )
    }
    fn max_scroll(&self) -> f32 {
        (self.layout.height - self.client_size().1 as f32 / self.scale).max(0.0)
    }
    fn current_hover(&self) -> Option<&str> {
        self.focus
            .and_then(|i| self.layout.hits.get(i))
            .and_then(|h| h.group.as_deref())
            .or(self.hover.as_deref())
    }
    fn tick(&mut self) {
        if self.demo {
            return;
        }
        let now = self.created.elapsed().as_secs() as i64;
        for config in &self.configs {
            if self.settings.disabled.contains(&config.id)
                || !self.schedule.reserve(&config.id, now)
            {
                continue;
            }
            let config = config.clone();
            let tx = self.tx.clone();
            let handle = self.hwnd.0 as usize;
            let token = Cancellation::default();
            self.tokens.insert(config.id.clone(), token.clone());
            self.workers.push(std::thread::spawn(move || {
                let runs = runner::run(&config, &token);
                let _ = tx.send(Completion {
                    id: config.id,
                    interval: config.interval,
                    runs,
                });
                unsafe {
                    let _ = PostMessageW(
                        Some(HWND(handle as *mut c_void)),
                        UPDATE,
                        WPARAM(0),
                        LPARAM(0),
                    );
                }
            }));
        }
        self.workers.retain(|w| !w.is_finished());
    }
    fn complete(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            self.tokens.remove(&result.id);
            let ok = result
                .runs
                .iter()
                .all(|r| r.result.as_ref().is_some_and(Result::is_ok));
            self.schedule.finish(
                &result.id,
                self.created.elapsed().as_secs() as i64,
                result.interval,
                ok,
            );
            let cancelled = result.runs.iter().any(|r| {
                r.result
                    .as_ref()
                    .is_some_and(|v| v.as_ref().is_err_and(|e| e.contains("cancelled")))
            });
            if !cancelled
                && self.configs.iter().any(|c| c.id == result.id)
                && !self.settings.disabled.contains(&result.id)
            {
                self.runs.retain(|r| r.config_id != result.id);
                self.runs.extend(result.runs);
                self.updated = platform::format_time(relative_time::now());
            }
        }
        self.icon(false);
        self.save();
        if self.visible {
            self.show();
        }
        self.tick();
    }
    fn refresh(&mut self) {
        if self.demo {
            self.runs = self.configs.iter().map(demo_run).collect();
            self.updated = platform::format_time(relative_time::now());
        } else {
            let (configs, errors) = store::load(Some(&self.folder.join("providers")));
            self.configs = configs;
            self.errors = errors;
            self.runs
                .retain(|r| self.configs.iter().any(|c| c.id == r.config_id));
            for c in &self.configs {
                if !self.runs.iter().any(|r| r.config_id == c.id) {
                    self.runs.push(Run::loading(c));
                }
            }
            self.schedule.refresh();
            self.tick();
        }
        self.icon(false);
        if self.visible {
            self.show();
        }
    }
    fn action(&mut self, action: Action) {
        match action {
            Action::Pin(pin) => {
                self.settings.pin = Some(pin);
                self.hide();
                self.icon(false);
            }
            Action::Hide(id) => {
                Settings::toggle(&mut self.settings.hidden, &id);
                self.hide();
            }
            Action::Collapse(id) => {
                Settings::toggle(&mut self.settings.collapsed, &id);
                self.hide();
            }
            Action::Move(id, step) => {
                card_order::move_visible(
                    &mut self.settings.order,
                    &id,
                    step,
                    &self.settings.hidden,
                );
                self.hide();
            }
            Action::Refresh => self.refresh(),
            Action::Providers => self.popup(1),
            Action::Hidden => self.popup(2),
            Action::Login => {
                if !self.demo && platform::set_login(!platform::login_enabled()).is_err() {
                    self.errors.push("Could not update Launch at Login".into());
                }
                self.hide();
            }
            Action::Folder => {
                platform::open_folder(&self.folder.join("providers"));
                self.hide();
            }
            Action::Quit => unsafe {
                let _ = PostMessageW(Some(self.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            },
            Action::DismissHint => {
                self.settings.overflow_hint_dismissed = true;
                self.show();
            }
        }
        self.save();
    }
    fn popup(&mut self, mode: u8) {
        unsafe {
            struct Menu(HMENU);
            impl Drop for Menu {
                fn drop(&mut self) {
                    unsafe {
                        let _ = DestroyMenu(self.0);
                    }
                }
            }
            let Ok(raw) = CreatePopupMenu() else {
                return;
            };
            let menu = Menu(raw);
            let mut commands = Vec::<(u32, Action)>::new();
            let add = |menu: HMENU,
                       text: &str,
                       action: Action,
                       checked: bool,
                       commands: &mut Vec<(u32, Action)>| {
                let id = 100 + commands.len() as u32;
                let text = platform::wide(&text.replace('&', "&&"));
                let _ = AppendMenuW(
                    menu,
                    MF_STRING | if checked { MF_CHECKED } else { MF_UNCHECKED },
                    id as usize,
                    PCWSTR(text.as_ptr()),
                );
                commands.push((id, action));
            };
            if mode == 1 {
                for c in &self.configs {
                    add(
                        menu.0,
                        &c.name,
                        Action::Pin(format!("@provider:{}", c.id)),
                        !self.settings.disabled.contains(&c.id),
                        &mut commands,
                    );
                }
            } else if mode == 2 {
                for id in &self.settings.hidden {
                    let name = self
                        .runs
                        .iter()
                        .find(|r| &r.id == id)
                        .map(|r| r.name.as_str())
                        .unwrap_or(id);
                    add(
                        menu.0,
                        &format!("Show {name}"),
                        Action::Pin(format!("@show:{id}")),
                        false,
                        &mut commands,
                    );
                }
            } else {
                add(menu.0, "Refresh Now", Action::Refresh, false, &mut commands);
                if let Ok(sub) = CreatePopupMenu() {
                    for c in &self.configs {
                        add(
                            sub,
                            &c.name,
                            Action::Pin(format!("@provider:{}", c.id)),
                            !self.settings.disabled.contains(&c.id),
                            &mut commands,
                        );
                    }
                    let _ = AppendMenuW(menu.0, MF_POPUP, sub.0 as usize, w!("Providers"));
                }
                if !self.settings.hidden.is_empty() {
                    if let Ok(sub) = CreatePopupMenu() {
                        for id in &self.settings.hidden {
                            add(
                                sub,
                                &format!("Show {id}"),
                                Action::Pin(format!("@show:{id}")),
                                false,
                                &mut commands,
                            );
                        }
                        let _ = AppendMenuW(menu.0, MF_POPUP, sub.0 as usize, w!("Hidden Cards"));
                    }
                }
                add(
                    menu.0,
                    "Launch at Login",
                    Action::Login,
                    !self.demo && platform::login_enabled(),
                    &mut commands,
                );
                add(
                    menu.0,
                    "Open Providers Folder…",
                    Action::Folder,
                    false,
                    &mut commands,
                );
                let version = platform::wide(&format!("AI Usage {}", env!("CARGO_PKG_VERSION")));
                let _ = AppendMenuW(menu.0, MF_STRING | MF_DISABLED, 0, PCWSTR(version.as_ptr()));
                add(menu.0, "Quit AI Usage", Action::Quit, false, &mut commands);
            }
            let mut pos = POINT::default();
            let _ = GetCursorPos(&mut pos);
            let _ = SetForegroundWindow(self.hwnd);
            let selected = TrackPopupMenuEx(
                menu.0,
                (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON).0,
                pos.x,
                pos.y,
                self.hwnd,
                None,
            )
            .0 as u32;
            let _ = PostMessageW(Some(self.hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            if let Some((_, action)) = commands.into_iter().find(|(id, _)| *id == selected) {
                if let Action::Pin(s) = &action {
                    if let Some(id) = s.strip_prefix("@provider:") {
                        Settings::toggle(&mut self.settings.disabled, id);
                        if self.settings.disabled.iter().any(|s| s == id) {
                            if let Some(token) = self.tokens.get(id) {
                                token.cancel();
                            }
                        } else {
                            self.schedule.refresh();
                            self.tick();
                        }
                        self.hide();
                        self.icon(false);
                        self.save();
                        return;
                    }
                    if let Some(id) = s.strip_prefix("@show:") {
                        self.settings.hidden.retain(|s| s != id);
                        self.save();
                        self.show();
                        return;
                    }
                }
                self.action(action);
            }
        }
    }
    fn keyboard(&mut self, key: u32) {
        if key == 27 {
            self.hide();
            return;
        }
        if key == 116 || (key == 0x52 && unsafe { GetKeyState(17) } < 0) {
            self.refresh();
            return;
        }
        if key == 9 && !self.layout.hits.is_empty() {
            let back = unsafe { GetKeyState(16) } < 0;
            let n = self.layout.hits.len();
            self.focus = Some(match self.focus {
                None => 0,
                Some(i) => {
                    if back {
                        (i + n - 1) % n
                    } else {
                        (i + 1) % n
                    }
                }
            });
            let h = &self.layout.hits[self.focus.unwrap()];
            let height = self.client_size().1 as f32 / self.scale;
            if h.rect.y < self.scroll {
                self.scroll = h.rect.y.max(0.0);
            }
            if h.rect.y + h.rect.h > self.scroll + height {
                self.scroll = (h.rect.y + h.rect.h - height + 8.0).min(self.max_scroll());
            }
            self.repaint();
        }
        if key == 13 || key == 32 {
            if let Some(action) = self
                .focus
                .and_then(|i| self.layout.hits.get(i))
                .map(|h| h.action.clone())
            {
                self.action(action);
            }
        }
    }
}
impl Drop for App {
    fn drop(&mut self) {
        for token in self.tokens.values() {
            token.cancel();
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
        self.save();
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = &*(lparam.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }
    if message == WM_CLOSE {
        let _ = DestroyWindow(hwnd);
        return LRESULT(0);
    }
    if message == WM_DESTROY {
        let _ = KillTimer(Some(hwnd), 1);
        PostQuitMessage(0);
        return LRESULT(0);
    }
    if message == WM_NCDESTROY {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<App>;
    let Some(cell) = pointer.as_ref() else {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    };
    let Ok(mut app) = cell.try_borrow_mut() else {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    };
    if message == app.taskbar_message && message != 0 {
        app.icon(true);
        return LRESULT(0);
    }
    match message {
        UPDATE => {
            app.complete();
            LRESULT(0)
        }
        TRAY => {
            let code = (lparam.0 as u32) & 0xffff;
            if code == WM_CONTEXTMENU || code == WM_RBUTTONUP {
                app.popup(0);
            } else if ai_usage_core::tray_event::activates_panel(lparam.0) {
                if app.visible {
                    app.hide();
                } else {
                    app.show();
                }
            }
            LRESULT(0)
        }
        WM_TIMER => {
            app.tick();
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut paint);
            let pixels = app.client_size();
            let hover = app.current_hover().map(str::to_string);
            let layout = app.layout.clone();
            let scale = app.scale;
            let scroll = app.scroll;
            let focus = app.focus;
            let hover_hit = app.hover_hit;
            let _ = app.painter.paint(
                hwnd,
                pixels,
                scale,
                &layout,
                scroll,
                hover.as_deref(),
                focus,
                hover_hit,
                platform::dark(),
            );
            let _ = EndPaint(hwnd, &paint);
            LRESULT(0)
        }
        WM_SIZE => {
            let width = (lparam.0 as u32) & 0xffff;
            let height = (lparam.0 as u32 >> 16) & 0xffff;
            if width > 0 && height > 0 {
                app.painter.resize(width, height);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let x = ((lparam.0 as u32 & 0xffff) as i16) as f32 / app.scale;
            let y = ((lparam.0 as u32 >> 16) as i16) as f32 / app.scale + app.scroll;
            let hover = app
                .layout
                .cards
                .iter()
                .find(|(_, r)| r.contains(x, y))
                .map(|(id, _)| id.clone());
            let hover_hit = app
                .layout
                .hits
                .iter()
                .enumerate()
                .rev()
                .find(|(_, h)| h.rect.contains(x, y) && h.visibility.shown(hover.as_deref()))
                .map(|(i, _)| i);
            if app.hover != hover || app.hover_hit != hover_hit || app.focus.is_some() {
                app.hover = hover;
                app.hover_hit = hover_hit;
                app.focus = None;
                app.repaint();
            }
            let mut track = TRACKMOUSEEVENT {
                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            let _ = TrackMouseEvent(&mut track);
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            app.hover = None;
            app.hover_hit = None;
            app.repaint();
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let x = ((lparam.0 as u32 & 0xffff) as i16) as f32 / app.scale;
            let y = ((lparam.0 as u32 >> 16) as i16) as f32 / app.scale + app.scroll;
            let action = app
                .layout
                .hits
                .iter()
                .rev()
                .find(|h| h.rect.contains(x, y) && h.visibility.shown(app.current_hover()))
                .map(|h| h.action.clone());
            if let Some(action) = action {
                app.action(action);
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            app.hover_hit = None;
            let delta = ((wparam.0 >> 16) as u16) as i16;
            app.scroll = (app.scroll - delta as f32 / 120.0 * 70.0).clamp(0.0, app.max_scroll());
            app.repaint();
            LRESULT(0)
        }
        WM_KEYDOWN => {
            app.keyboard(wparam.0 as u32);
            LRESULT(0)
        }
        WM_ACTIVATE => {
            if wparam.0 & 0xffff == 0 && app.visible && !app.demo {
                app.hide();
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            app.scale = (wparam.0 & 0xffff) as f32 / 96.0;
            app.painter.target = None;
            app.icon(false);
            if app.visible {
                app.show();
            }
            LRESULT(0)
        }
        WM_SETTINGCHANGE | WM_THEMECHANGED => {
            app.painter.target = None;
            app.icon(false);
            app.repaint();
            LRESULT(0)
        }
        WM_QUERYENDSESSION => LRESULT(1),
        WM_ENDSESSION => {
            if wparam.0 != 0 {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}
fn demo_run(config: &ProviderConfig) -> Run {
    let now = relative_time::now();
    let window =
        |id: &str, label: Option<&str>, percent: f64, after: i64, duration: Option<i64>| Window {
            id: id.into(),
            label: label.map(str::to_string),
            used: Some(percent),
            resets_at: Some(now + after),
            duration,
            count: None,
        };
    let mut run = Run::loading(config);
    let meters = match config.id.as_str() {
        "claude" => vec![Meter {
            id: "plan".into(),
            label: "Plan".into(),
            windows: vec![
                window("five_hour", Some("Session"), 42.0, 7200, Some(18000)),
                window("seven_day", Some("Weekly"), 67.0, 4 * 86400, Some(604800)),
            ],
        }],
        "codex" => vec![Meter {
            id: "codex".into(),
            label: "Codex".into(),
            windows: vec![
                window("primary", Some("Session"), 26.0, 14400, Some(18000)),
                window("secondary", Some("Weekly"), 82.0, 2 * 86400, Some(604800)),
            ],
        }],
        "antigravity" => vec![
            Meter {
                id: "Gemini Models".into(),
                label: "Gemini Models".into(),
                windows: vec![
                    window("five_hour", Some("Five Hour"), 63.0, 10800, Some(18000)),
                    window("weekly", Some("Weekly"), 7.0, 4 * 86400, Some(604800)),
                ],
            },
            Meter {
                id: "Claude and GPT models".into(),
                label: "Claude and GPT models".into(),
                windows: vec![window(
                    "weekly",
                    Some("Weekly"),
                    92.0,
                    4 * 86400,
                    Some(604800),
                )],
            },
        ],
        _ => {
            run.id = "copilot:sample-account".into();
            run.account = Some("sample-account".into());
            let mut w = window("current", None, 18.0, 21 * 86400, None);
            w.count = Some(Count {
                used: 54.0,
                limit: 300.0,
                unit: None,
            });
            vec![Meter {
                id: "premium_interactions".into(),
                label: "Premium Interactions".into(),
                windows: vec![w],
            }]
        }
    };
    run.result = Some(Ok(Report {
        plan: match config.id.as_str() {
            "claude" => Some("max".into()),
            "codex" => Some("plus".into()),
            "copilot" => Some("pro".into()),
            _ => None,
        },
        available: None,
        access: None,
        meters,
    }));
    run
}
fn gui(demo: bool) -> windows::core::Result<()> {
    unsafe {
        let name = if demo {
            w!("Local\\g.akrp.AIUsage.Windows.Demo")
        } else {
            w!("Local\\g.akrp.AIUsage.Windows")
        };
        let mutex = ai_usage_core::process::Handle(CreateMutexW(None, false, name)?);
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return Ok(());
        }
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        struct Com;
        impl Drop for Com {
            fn drop(&mut self) {
                unsafe {
                    CoUninitialize();
                }
            }
        }
        let _com = Com;
        let module = GetModuleHandleW(None)?;
        let class = w!("AIUsageWindowsNative");
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: module.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: class,
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            return Err(windows::core::Error::from_thread());
        }
        let app = Box::new(RefCell::new(App::new(demo)?));
        let extended = if demo {
            WS_EX_APPWINDOW | WS_EX_TOPMOST
        } else {
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST
        };
        let hwnd = CreateWindowExW(
            extended,
            class,
            w!("AI Usage"),
            WS_POPUP,
            0,
            0,
            320,
            600,
            None,
            None,
            Some(module.into()),
            Some((&*app as *const RefCell<App>).cast()),
        )?;
        let preference = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&preference as *const DWM_WINDOW_CORNER_PREFERENCE).cast(),
            std::mem::size_of_val(&preference) as u32,
        );
        {
            let mut app = app.borrow_mut();
            app.hwnd = hwnd;
            app.scale = (GetDpiForWindow(hwnd) as f32 / 96.0).max(1.0);
            app.taskbar_message = RegisterWindowMessageW(w!("TaskbarCreated"));
            app.tray = Some(platform::Tray::new(hwnd, TRAY));
            app.icon(true);
            app.tick();
            if demo {
                app.show();
            }
        }
        if SetTimer(Some(hwnd), 1, 30000, None) == 0 {
            let _ = DestroyWindow(hwnd);
            return Err(windows::core::Error::from_thread());
        }
        let mut message = MSG::default();
        loop {
            let result = GetMessageW(&mut message, None, 0, 0).0;
            if result <= 0 {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        drop(app);
        drop(mutex);
        Ok(())
    }
}
fn main() {
    std::panic::set_hook(Box::new(|_| {
        let dir = settings::folder(false);
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(
            dir.join("last-error.txt"),
            "AI Usage encountered an internal error\n",
        );
    }));
    let args = std::env::args().collect::<Vec<_>>();
    if args.iter().any(|a| a == "--version") {
        println!("AI Usage {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.iter().any(|a| a == "--live") {
        let preferences = settings::load(&settings::folder(false));
        let (configs, errors) = store::load(Some(&settings::folder(false).join("providers")));
        for e in errors {
            eprintln!("Configuration: {e}");
        }
        for config in configs {
            if preferences.disabled.contains(&config.id) {
                continue;
            }
            for run in runner::run(&config, &Cancellation::default()) {
                match run.result {
                    Some(Ok(report)) => println!(
                        "{}: headline={:?}, max={:?}, meters={}, available={:?}",
                        run.name,
                        report.headline(),
                        report.max_percent(),
                        report.meters.len(),
                        report.available
                    ),
                    Some(Err(e)) => println!("{}: {e}", run.name),
                    None => {}
                }
            }
        }
        return;
    }
    if let Err(error) = gui(args.iter().any(|a| a == "--demo")) {
        let text = platform::wide(&format!(
            "AI Usage could not start ({:08x})",
            error.code().0 as u32
        ));
        unsafe {
            MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                w!("AI Usage"),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}
