//! Inspect or drag only an explicitly owned au2 host on DISPLAY2.
#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    use std::ffi::c_void;
    type H = *mut c_void;
    #[repr(C)]
    #[derive(Default, Copy, Clone)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    struct BitmapInfo {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bits: u16,
        compression: u32,
        image_size: u32,
        x: i32,
        y: i32,
        used: u32,
        important: u32,
    }
    #[repr(C)]
    struct MonitorInfo {
        size: u32,
        rect: Rect,
        work: Rect,
        flags: u32,
        device: [u16; 32],
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumWindows(cb: unsafe extern "system" fn(H, isize) -> i32, data: isize) -> i32;
        fn EnumChildWindows(
            w: H,
            cb: unsafe extern "system" fn(H, isize) -> i32,
            data: isize,
        ) -> i32;
        fn GetWindowThreadProcessId(w: H, p: *mut u32) -> u32;
        fn GetWindowTextW(w: H, s: *mut u16, n: i32) -> i32;
        fn GetClassNameW(w: H, s: *mut u16, n: i32) -> i32;
        fn GetWindowRect(w: H, r: *mut Rect) -> i32;
        fn GetClientRect(w: H, r: *mut Rect) -> i32;
        fn ClientToScreen(w: H, p: *mut [i32; 2]) -> i32;
        fn GetMenu(w: H) -> H;
        fn SendMessageW(w: H, m: u32, wp: usize, lp: isize) -> isize;
        fn PrintWindow(w: H, dc: H, flags: u32) -> i32;
        fn GetDC(w: H) -> H;
        fn ReleaseDC(w: H, dc: H) -> i32;
        fn SetProcessDpiAwarenessContext(c: isize) -> i32;
        fn MonitorFromWindow(w: H, flags: u32) -> H;
        fn GetMonitorInfoW(m: H, info: *mut MonitorInfo) -> i32;
    }
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateCompatibleDC(dc: H) -> H;
        fn CreateDIBSection(
            dc: H,
            info: *const BitmapInfo,
            usage: u32,
            bits: *mut *mut c_void,
            section: H,
            offset: u32,
        ) -> H;
        fn SelectObject(dc: H, o: H) -> H;
        fn DeleteObject(o: H) -> i32;
        fn DeleteDC(dc: H) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> H;
        fn QueryFullProcessImageNameW(p: H, flags: u32, s: *mut u16, n: *mut u32) -> i32;
        fn CloseHandle(h: H) -> i32;
    }
    struct State {
        pid: u32,
        root: H,
        windows: Vec<serde_json::Value>,
    }
    unsafe fn name(w: H, class: bool) -> String {
        let mut b = [0u16; 1024];
        let n = unsafe {
            if class {
                GetClassNameW(w, b.as_mut_ptr(), 1024)
            } else {
                GetWindowTextW(w, b.as_mut_ptr(), 1024)
            }
        };
        String::from_utf16_lossy(&b[..n.max(0) as usize])
    }
    unsafe extern "system" fn collect(w: H, ctx: isize) -> i32 {
        unsafe {
            let s = &mut *(ctx as *mut State);
            let mut pid = 0;
            GetWindowThreadProcessId(w, &mut pid);
            if pid == s.pid {
                let mut r = Rect::default();
                GetWindowRect(w, &mut r);
                s.windows.push(serde_json::json!({"hwnd":w as usize,"title":name(w,false),"class":name(w,true),"rect":[r.left,r.top,r.right,r.bottom]}));
                if !GetMenu(w).is_null() {
                    s.root = w;
                }
            }
        }
        1
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        args.len() == 2 || args.len() == 7,
        "movement_gui <pid> <output.png> [hwnd x y dx dy]"
    );
    let pid = args[0].parse()?;
    unsafe {
        SetProcessDpiAwarenessContext(-4);
        let p = OpenProcess(0x1000, 0, pid);
        anyhow::ensure!(!p.is_null(), "Cannot inspect explicit PID");
        let mut b = [0u16; 32768];
        let mut n = b.len() as u32;
        let ok = QueryFullProcessImageNameW(p, 0, b.as_mut_ptr(), &mut n);
        CloseHandle(p);
        anyhow::ensure!(ok != 0, "Cannot verify host path");
        let expected = std::fs::canonicalize(".aviutl2-cli/development/aviutl2.exe")?;
        anyhow::ensure!(
            std::fs::canonicalize(String::from_utf16_lossy(&b[..n as usize]))? == expected,
            "Refused non-development PID"
        );
        let mut s = State {
            pid,
            root: std::ptr::null_mut(),
            windows: Vec::new(),
        };
        EnumWindows(collect, &mut s as *mut _ as isize);
        anyhow::ensure!(!s.root.is_null(), "Host main window not found");
        EnumChildWindows(s.root, collect, &mut s as *mut _ as isize);
        let mut bounds = Rect::default();
        GetWindowRect(s.root, &mut bounds);
        let mut origin = [0, 0];
        ClientToScreen(s.root, &mut origin);
        println!("root={} client_origin={origin:?}", s.root as usize);
        let monitor = MonitorFromWindow(s.root, 0);
        let mut info: MonitorInfo = std::mem::zeroed();
        info.size = std::mem::size_of::<MonitorInfo>() as u32;
        anyhow::ensure!(
            GetMonitorInfoW(monitor, &mut info) != 0,
            "Cannot verify monitor"
        );
        let length = info.device.iter().position(|c| *c == 0).unwrap_or(32);
        anyhow::ensure!(
            String::from_utf16_lossy(&info.device[..length]) == r"\\.\DISPLAY2",
            "Host is outside DISPLAY2"
        );
        anyhow::ensure!(
            bounds.left >= info.work.left
                && bounds.top >= info.work.top
                && bounds.right <= info.work.right
                && bounds.bottom <= info.work.bottom,
            "Host is outside DISPLAY2"
        );
        std::fs::write(
            std::path::Path::new(&args[1]).with_extension("windows.json"),
            serde_json::to_vec_pretty(&s.windows)?,
        )?;
        if args.len() == 7 {
            let hwnd = args[2].parse::<usize>()? as H;
            anyhow::ensure!(
                s.windows
                    .iter()
                    .any(|v| v["hwnd"].as_u64() == Some(hwnd as u64)),
                "Refused unowned window"
            );
            let x = args[3].parse::<i32>()?;
            let y = args[4].parse::<i32>()?;
            let dx = args[5].parse::<i32>()?;
            let dy = args[6].parse::<i32>()?;
            let mut r = Rect::default();
            GetClientRect(hwnd, &mut r);
            anyhow::ensure!(
                [x, x + dx].iter().all(|v| *v >= 0 && *v < r.right)
                    && [y, y + dy].iter().all(|v| *v >= 0 && *v < r.bottom),
                "Drag outside owned client"
            );
            let point = |x: i32, y: i32| ((x as u32 & 65535) | ((y as u32 & 65535) << 16)) as isize;
            SendMessageW(hwnd, 0x201, 1, point(x, y));
            for i in 1..=10 {
                SendMessageW(hwnd, 0x200, 1, point(x + dx * i / 10, y + dy * i / 10));
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            SendMessageW(hwnd, 0x202, 0, point(x + dx, y + dy));
            std::thread::sleep(std::time::Duration::from_millis(600));
        }
        let w = bounds.right - bounds.left;
        let h = bounds.bottom - bounds.top;
        let info = BitmapInfo {
            size: 40,
            width: w,
            height: -h,
            planes: 1,
            bits: 32,
            compression: 0,
            image_size: 0,
            x: 0,
            y: 0,
            used: 0,
            important: 0,
        };
        let source = GetDC(s.root);
        let dc = CreateCompatibleDC(source);
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(source, &info, 0, &mut bits, std::ptr::null_mut(), 0);
        anyhow::ensure!(
            !bitmap.is_null() && !dc.is_null(),
            "Cannot capture owned window"
        );
        let previous = SelectObject(dc, bitmap);
        let painted = PrintWindow(s.root, dc, 2);
        let mut rgba = std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize).to_vec();
        SelectObject(dc, previous);
        DeleteObject(bitmap);
        DeleteDC(dc);
        ReleaseDC(s.root, source);
        anyhow::ensure!(painted != 0, "PrintWindow failed");
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }
        image::save_buffer(&args[1], &rgba, w as u32, h as u32, image::ColorType::Rgba8)?;
    }
    Ok(())
}
#[cfg(not(windows))]
fn main() {
    panic!("Windows only")
}
