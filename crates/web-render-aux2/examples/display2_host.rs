//! Start and supervise only the isolated development host on Windows DISPLAY2.
#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    use std::{ffi::c_void, mem::size_of, os::windows::process::CommandExt};
    type H = *mut c_void;
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    struct Monitor {
        size: u32,
        rect: Rect,
        work: Rect,
        flags: u32,
        device: [u16; 32],
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetProcessDpiAwarenessContext(context: isize) -> i32;
        fn EnumDisplayMonitors(
            dc: H,
            clip: *const Rect,
            cb: unsafe extern "system" fn(H, H, *mut Rect, isize) -> i32,
            data: isize,
        ) -> i32;
        fn GetMonitorInfoW(monitor: H, info: *mut Monitor) -> i32;
        fn EnumWindows(cb: unsafe extern "system" fn(H, isize) -> i32, data: isize) -> i32;
        fn EnumChildWindows(
            hwnd: H,
            cb: unsafe extern "system" fn(H, isize) -> i32,
            data: isize,
        ) -> i32;
        fn GetWindowThreadProcessId(hwnd: H, pid: *mut u32) -> u32;
        fn GetWindowTextW(hwnd: H, text: *mut u16, count: i32) -> i32;
        fn GetClassNameW(hwnd: H, text: *mut u16, count: i32) -> i32;
        fn GetWindowRect(hwnd: H, rect: *mut Rect) -> i32;
        fn IsWindowVisible(hwnd: H) -> i32;
        fn SetWindowPos(hwnd: H, after: H, x: i32, y: i32, w: i32, h: i32, flags: u32) -> i32;
        fn GetWindow(hwnd: H, cmd: u32) -> H;
        fn PostMessageW(hwnd: H, message: u32, wparam: usize, lparam: isize) -> i32;
        fn GetMenu(hwnd: H) -> H;
        fn GetSubMenu(menu: H, index: i32) -> H;
        fn GetMenuItemCount(menu: H) -> i32;
        fn GetMenuItemID(menu: H, index: i32) -> u32;
        fn GetMenuStringW(menu: H, index: u32, text: *mut u16, count: i32, flags: u32) -> i32;
        fn SetWindowTextW(hwnd: H, text: *const u16) -> i32;
        fn SetForegroundWindow(hwnd: H) -> i32;
        fn GetForegroundWindow() -> H;
        fn AttachThreadInput(from: u32, to: u32, attach: i32) -> i32;
        fn SetFocus(hwnd: H) -> H;
        fn SetCursorPos(x: i32, y: i32) -> i32;
        fn mouse_event(flags: u32, x: u32, y: u32, data: u32, extra: usize);
        fn WindowFromPoint(point: i64) -> H;
        fn GetDlgCtrlID(hwnd: H) -> i32;
        fn IsWindowEnabled(hwnd: H) -> i32;
        fn SendMessageW(hwnd: H, message: u32, wparam: usize, lparam: isize) -> isize;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThreadId() -> u32;
    }
    unsafe extern "system" fn monitors(m: H, _: H, _: *mut Rect, ctx: isize) -> i32 {
        unsafe {
            let v = &mut *(ctx as *mut Vec<(String, Rect)>);
            let mut info: Monitor = std::mem::zeroed();
            info.size = size_of::<Monitor>() as u32;
            if GetMonitorInfoW(m, &mut info) != 0 {
                let n = info.device.iter().position(|c| *c == 0).unwrap_or(32);
                v.push((String::from_utf16_lossy(&info.device[..n]), info.work));
            }
        }
        1
    }
    struct Supervisor {
        pid: u32,
        work: Rect,
        expected: String,
        allowed: Vec<String>,
        moves: u32,
        windows: Vec<String>,
        trusted: bool,
        saved: bool,
        export: Option<std::path::PathBuf>,
        export_posted: bool,
        export_dialog_posted: bool,
    }
    struct Dialog {
        items: Vec<(H, String, String)>,
    }
    unsafe fn png_command(menu: H) -> Option<u32> {
        unsafe {
            if menu.is_null() {
                return None;
            }
            for i in 0..GetMenuItemCount(menu) {
                let mut name = [0u16; 1024];
                let n = GetMenuStringW(menu, i as u32, name.as_mut_ptr(), 1024, 0x400);
                let name = String::from_utf16_lossy(&name[..n.max(0) as usize]);
                if name.contains("PNGファイル出力") {
                    let id = GetMenuItemID(menu, i);
                    if id != u32::MAX {
                        return Some(id);
                    }
                }
                if let Some(id) = png_command(GetSubMenu(menu, i)) {
                    return Some(id);
                }
            }
        }
        None
    }
    unsafe fn text(hwnd: H, class: bool) -> String {
        let mut buffer = [0u16; 4096];
        let n = unsafe {
            if class {
                GetClassNameW(hwnd, buffer.as_mut_ptr(), 4096)
            } else {
                GetWindowTextW(hwnd, buffer.as_mut_ptr(), 4096)
            }
        };
        String::from_utf16_lossy(&buffer[..n.max(0) as usize])
    }
    unsafe extern "system" fn child(hwnd: H, ctx: isize) -> i32 {
        unsafe {
            (&mut *(ctx as *mut Dialog))
                .items
                .push((hwnd, text(hwnd, true), text(hwnd, false)));
        }
        1
    }
    unsafe extern "system" fn window(hwnd: H, ctx: isize) -> i32 {
        unsafe {
            let s = &mut *(ctx as *mut Supervisor);
            let mut owner = 0;
            GetWindowThreadProcessId(hwnd, &mut owner);
            if owner != s.pid {
                return 1;
            }
            let mut r = Rect::default();
            if GetWindowRect(hwnd, &mut r) == 0 || r.right <= r.left || r.bottom <= r.top {
                return 1;
            }
            let w = (r.right - r.left).min(s.work.right - s.work.left - 16);
            let h = (r.bottom - r.top).min(s.work.bottom - s.work.top - 16);
            if r.left < s.work.left
                || r.top < s.work.top
                || r.right > s.work.right
                || r.bottom > s.work.bottom
            {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    s.work.left + 8,
                    s.work.top + 8,
                    w,
                    h,
                    0x0014,
                );
                s.moves += 1;
            }
            if IsWindowVisible(hwnd) == 0 {
                return 1;
            }
            let title = text(hwnd, false);
            if !s.windows.contains(&title) {
                println!("Display2 window: {title}");
                s.windows.push(title.clone());
            }
            if s.export.is_some()
                && !s.export_posted
                && title.contains(&s.expected)
                && let Some(id) = png_command(GetMenu(hwnd))
            {
                PostMessageW(hwnd, 0x0111, id as usize, 0);
                s.export_posted = true;
                println!("Opened built-in PNG output on display 2, command={id}");
            }
            if text(hwnd, true) != "#32770" {
                return 1;
            }
            let mut dialog = Dialog { items: Vec::new() };
            EnumChildWindows(hwnd, child, &mut dialog as *mut Dialog as isize);
            if let Some(path) = &s.export
                && title.contains("PNG")
                && !s.export_dialog_posted
            {
                println!("PNG dialog controls: {:?}", dialog.items);
                let edits: Vec<_> = dialog
                    .items
                    .iter()
                    .filter(|(_, c, _)| c == "Edit")
                    .collect();
                if let (Some(edit), Some((button, _, _))) = (
                    edits.first(),
                    dialog
                        .items
                        .iter()
                        .find(|(_, c, t)| c == "Button" && (t == "保存(&S)" || t == "保存")),
                ) {
                    let value: Vec<_> = path
                        .to_string_lossy()
                        .encode_utf16()
                        .chain(Some(0))
                        .collect();
                    SetWindowTextW(edit.0, value.as_ptr());
                    let empty = [0u16];
                    SetWindowTextW(edit.0, empty.as_ptr());
                    for character in value.iter().copied().take(value.len() - 1) {
                        SendMessageW(edit.0, 0x0102, character as usize, 1);
                    }
                    let edit_id = GetDlgCtrlID(edit.0);
                    SendMessageW(hwnd, 0x0468, edit_id as usize, value.as_ptr() as isize);
                    println!(
                        "PNG controls edit_id={edit_id}, save_id={}, enabled={}",
                        GetDlgCtrlID(*button),
                        IsWindowEnabled(*button)
                    );
                    // Deliver button input directly to the verified control; a foreground
                    // overlay in an automated desktop may intercept global mouse input.
                    PostMessageW(*button, 0x0201, 1, (10 << 16) | 10);
                    PostMessageW(*button, 0x0202, 0, (10 << 16) | 10);
                    let mut rect = Rect::default();
                    if GetWindowRect(*button, &mut rect) != 0
                        && rect.left >= s.work.left
                        && rect.top >= s.work.top
                        && rect.right <= s.work.right
                        && rect.bottom <= s.work.bottom
                    {
                        let current = GetCurrentThreadId();
                        let target = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
                        let foreground =
                            GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
                        AttachThreadInput(current, target, 1);
                        AttachThreadInput(current, foreground, 1);
                        let active = SetForegroundWindow(hwnd);
                        SetFocus(*button);
                        println!(
                            "PNG foreground={:?}, activate={active}",
                            GetForegroundWindow()
                        );
                        SetWindowPos(hwnd, -1isize as H, 0, 0, 0, 0, 0x43);
                        let x = (rect.left + rect.right) / 2;
                        let y = (rect.top + rect.bottom) / 2;
                        let point = (x as u32 as i64) | ((y as i64) << 32);
                        println!(
                            "PNG save hit={:?}, expected={button:?}, rect={rect:?}",
                            WindowFromPoint(point)
                        );
                        if WindowFromPoint(point) == *button {
                            SetCursorPos(x, y);
                            mouse_event(2, 0, 0, 0, 0);
                            mouse_event(4, 0, 0, 0, 0);
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            if WindowFromPoint(point) == *button {
                                mouse_event(2, 0, 0, 0, 0);
                                mouse_event(4, 0, 0, 0, 0);
                            }
                        }
                        PostMessageW(*button, 0x00f5, 0, 0);
                        AttachThreadInput(current, foreground, 0);
                        AttachThreadInput(current, target, 0);
                    } else {
                        PostMessageW(hwnd, 0x0111, 1, *button as isize);
                    }
                    s.export_dialog_posted = true;
                    println!("Requested PNG export to {}", path.display());
                }
            }
            if title == "スクリプト・プラグインの追加" && !s.trusted {
                let entries: Vec<_> = dialog
                    .items
                    .iter()
                    .filter(|(_, c, t)| {
                        c == "Static"
                            && (t.contains(".obj2") || t.contains(".aux2") || t.contains(".dll"))
                    })
                    .flat_map(|(_, _, t)| t.lines().map(str::trim).filter(|s| !s.is_empty()))
                    .collect();
                if !entries.is_empty()
                    && entries.iter().all(|e| s.allowed.iter().any(|a| a == e))
                    && let Some((button, _, _)) = dialog.items.iter().find(|(_, c, t)| {
                        c == "Button" && t == "このプラグイン・スクリプトを信頼して使用する"
                    })
                {
                    PostMessageW(*button, 0x00f5, 0, 0);
                    s.trusted = true;
                    println!("Trusted the generated scripts of this explicit test project");
                }
            }
            if title == "AviUtl ExEdit2"
                && !s.saved
                && text(GetWindow(hwnd, 4), false).contains(&s.expected)
                && dialog.items.iter().any(|(_, c, t)| {
                    c == "Static"
                        && t.replace('\r', "").trim()
                            == "現在の編集データは更新されています\nプロジェクトを保存しますか？"
                })
                && let Some((button, _, _)) = dialog
                    .items
                    .iter()
                    .find(|(_, c, t)| c == "Button" && t == "はい(&Y)")
            {
                PostMessageW(*button, 0x00f5, 0, 0);
                s.saved = true;
                println!("Saved and closed only {}", s.expected);
            }
        }
        1
    }
    // Physical monitor coordinates; no screen capture or input occurs on other displays.
    unsafe {
        SetProcessDpiAwarenessContext(-4);
    }
    let mut displays: Vec<(String, Rect)> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            monitors,
            &mut displays as *mut _ as isize,
        );
    }
    for (name, r) in &displays {
        println!("{name}: {r:?}");
    }
    let work = displays
        .iter()
        .find(|(name, _)| name == r"\\.\DISPLAY2")
        .ok_or_else(|| {
            anyhow::anyhow!("DISPLAY2 is not connected; refusing to launch on another display")
        })?
        .1;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() {
        return Ok(());
    }
    if args.len() == 3 && args[0] == "--complete-png" {
        let pid = args[1].to_string_lossy().parse()?;
        let export = std::path::absolute(std::path::PathBuf::from(&args[2]))?;
        anyhow::ensure!(
            export.starts_with(std::env::current_dir()?.join("docs/proofs/libprocessing-aviutl"))
                && export.file_name().unwrap() == "native-export.png",
            "Refused another PNG target"
        );
        let mut s = Supervisor {
            pid,
            work,
            expected: "libprocessing-sample.aup2".into(),
            allowed: Vec::new(),
            moves: 0,
            windows: Vec::new(),
            trusted: false,
            saved: true,
            export: Some(export),
            export_posted: true,
            export_dialog_posted: false,
        };
        unsafe {
            EnumWindows(window, &mut s as *mut Supervisor as isize);
        }
        anyhow::ensure!(s.export_dialog_posted, "No owned PNG dialog was found");
        return Ok(());
    }
    anyhow::ensure!(
        args.len() == 4,
        "display2_host <development-exe> <test.aup2> <project-dir> <report.json>"
    );
    let exe = std::fs::canonicalize(&args[0])?;
    let root = exe.parent().unwrap();
    anyhow::ensure!(
        root.ends_with(".aviutl2-cli/development") && exe.file_name().unwrap() == "aviutl2.exe",
        "Refused a non-development host"
    );
    let project = std::fs::canonicalize(&args[1])?;
    let manifest = std::path::PathBuf::from(&args[2]);
    let ini = root.join("data/aviutl2.ini");
    let contents = std::fs::read_to_string(&ini)?;
    let mut output = String::new();
    let mut in_window = false;
    for line in contents.lines() {
        if line.starts_with('[') {
            in_window = line == "[Window]";
        }
        if !in_window {
            output.push_str(line);
            output.push('\n');
        }
    }
    output.push_str(&format!(
        "[Window]\nx={}\ny={}\nw={}\nh={}\nzoom=0\n",
        work.left + 16,
        work.top + 16,
        (work.right - work.left - 32).min(1600),
        (work.bottom - work.top - 32).min(1000)
    ));
    std::fs::write(&ini, output)?;
    let mut allowed = vec![
        r"web-render\web-render.aux2".into(),
        r"web-render/web-render.aux2".into(),
    ];
    if manifest.join(web_render_processing::MANIFEST).is_file() {
        let native = web_render_processing::Project::load(&manifest)?;
        for object in native.objects {
            allowed.push(format!(
                "web-render.aux2_{}\\{}.obj2",
                native.name, object.label
            ));
        }
    } else if manifest.ends_with("himawari-receiver") {
        for label in ["Drum", "Kaiwai Phrase", "Synth Solo"] {
            allowed.push(format!("web-render.aux2_himawari-test\\{label}.obj2"));
        }
    } else if manifest.ends_with("p5js") {
        allowed.push(r"web-render.aux2_p5-midi-test\MIDI Pattern Grid.obj2".into());
    }
    let file =
        std::fs::File::create(std::path::PathBuf::from(&args[3]).with_extension("stdout.txt"))?;
    let mut host = std::process::Command::new(&exe)
        .arg(&project)
        .current_dir(root)
        .creation_flags(0x08000000)
        .stdout(file.try_clone()?)
        .stderr(file)
        .spawn()?;
    let mut s = Supervisor {
        pid: host.id(),
        work,
        expected: project.file_name().unwrap().to_string_lossy().into_owned(),
        allowed,
        moves: 0,
        windows: Vec::new(),
        trusted: false,
        saved: false,
        export: None,
        export_posted: false,
        export_dialog_posted: false,
    };
    println!("Development host PID={}", s.pid);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
    let smoke = std::env::var("WEB_RENDER_GUI_SMOKE").as_deref() == Ok("1");
    let smoke_seconds = std::env::var("WEB_RENDER_GUI_SMOKE_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(10)
        .clamp(10, 120);
    let smoke_close = std::time::Instant::now() + std::time::Duration::from_secs(smoke_seconds);
    let mut close_posted = false;
    let status = loop {
        if s.export.is_none() {
            let signal = std::path::PathBuf::from(&args[3]).with_file_name("export-request.json");
            if let Ok(bytes) = std::fs::read(&signal)
                && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
                && let Some(path) = value["file"].as_str()
            {
                let path = std::path::absolute(path)?;
                let allowed = std::path::absolute(
                    std::path::PathBuf::from(&args[3]).with_file_name("native-export.png"),
                )?;
                anyhow::ensure!(
                    path == allowed,
                    "Refused an export outside this explicit test output"
                );
                s.export = Some(path);
            }
        }
        unsafe {
            EnumWindows(window, &mut s as *mut Supervisor as isize);
        }
        if smoke && !close_posted && std::time::Instant::now() >= smoke_close {
            unsafe extern "system" fn close(hwnd: H, context: isize) -> i32 {
                unsafe {
                    let s = &*(context as *const Supervisor);
                    let mut pid = 0;
                    GetWindowThreadProcessId(hwnd, &mut pid);
                    if pid == s.pid
                        && text(hwnd, false).contains(&s.expected)
                        && !GetMenu(hwnd).is_null()
                    {
                        PostMessageW(hwnd, 0x0010, 0, 0);
                    }
                }
                1
            }
            unsafe {
                EnumWindows(close, &s as *const Supervisor as isize);
            }
            close_posted = true;
        }
        if let Some(status) = host.try_wait()? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            host.kill()?;
            anyhow::bail!("Development test exceeded deadline");
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    };
    let report = serde_json::json!({"display":"DISPLAY2","work_area":[work.left,work.top,work.right,work.bottom],"pid":s.pid,
        "exit_code":status.code(),"position_corrections":s.moves,"windows":s.windows,"trusted_generated_scripts":s.trusted,"saved_test_project":s.saved});
    std::fs::write(&args[3], serde_json::to_vec_pretty(&report)?)?;
    anyhow::ensure!(status.success(), "Development host failed: {status}");
    Ok(())
}
#[cfg(not(windows))]
fn main() {
    panic!("Windows only");
}
