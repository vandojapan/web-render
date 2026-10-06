//! GUI diagnostics; optional confirmation limited to explicitly named test scripts/projects.
#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    use std::{ffi::c_void, mem::size_of};
    type Handle = *mut c_void;
    struct Inspection {
        pid: u32,
        script: bool,
        other_script: bool,
        button: Handle,
        trust_dialog: bool,
        save_dialog: bool,
        save_question: bool,
        save_button: Handle,
        himawari: bool,
    }
    #[repr(C)]
    struct ThreadEntry {
        size: u32,
        usage: u32,
        id: u32,
        owner: u32,
        priority: i32,
        delta: i32,
        flags: u32,
    }
    #[repr(C)]
    struct GuiInfo {
        size: u32,
        flags: u32,
        active: Handle,
        focus: Handle,
        capture: Handle,
        menu: Handle,
        moving: Handle,
        caret: Handle,
        rect: [i32; 4],
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> Handle;
        fn Thread32First(snapshot: Handle, entry: *mut ThreadEntry) -> i32;
        fn Thread32Next(snapshot: Handle, entry: *mut ThreadEntry) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
        fn QueryFullProcessImageNameW(
            process: Handle,
            flags: u32,
            path: *mut u16,
            length: *mut u32,
        ) -> i32;
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetGUIThreadInfo(thread: u32, info: *mut GuiInfo) -> i32;
        fn GetWindowThreadProcessId(window: Handle, pid: *mut u32) -> u32;
        fn GetWindowTextW(window: Handle, text: *mut u16, length: i32) -> i32;
        fn GetClassNameW(window: Handle, text: *mut u16, length: i32) -> i32;
        fn PostMessageW(window: Handle, message: u32, wparam: usize, lparam: isize) -> i32;
        fn GetWindow(window: Handle, command: u32) -> Handle;
        fn EnumChildWindows(
            window: Handle,
            callback: unsafe extern "system" fn(Handle, isize) -> i32,
            param: isize,
        ) -> i32;
    }
    unsafe extern "system" fn print_window(window: Handle, context: isize) -> i32 {
        let mut owner = 0;
        let mut title = [0u16; 1024];
        let mut class = [0u16; 128];
        // SAFETY: initialized output buffers; only windows owned by the verified PID are read.
        unsafe {
            // context points to the live Inspection during synchronous enumeration.
            let inspection = &mut *(context as *mut Inspection);
            GetWindowThreadProcessId(window, &mut owner);
            if owner == inspection.pid {
                let length = GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32);
                let class_length = GetClassNameW(window, class.as_mut_ptr(), class.len() as i32);
                let title = String::from_utf16_lossy(&title[..length.max(0) as usize]);
                let class = String::from_utf16_lossy(&class[..class_length.max(0) as usize]);
                println!("hwnd={window:?}, class={}, text={}", class, title);
                if class == "#32770" && title == "スクリプト・プラグインの追加" {
                    inspection.trust_dialog = true;
                }
                if class == "#32770" && title == "AviUtl ExEdit2" {
                    inspection.save_dialog = true;
                }
                if class == "Static"
                    && title.replace('\r', "").trim()
                        == "現在の編集データは更新されています\nプロジェクトを保存しますか？"
                {
                    inspection.save_question = true;
                }
                if class == "Button" && title == "はい(&Y)" {
                    inspection.save_button = window;
                }
                let allowed = if inspection.himawari {
                    &[
                        r"web-render.aux2_himawari-test\Drum.obj2",
                        r"web-render.aux2_himawari-test\Kaiwai Phrase.obj2",
                        r"web-render.aux2_himawari-test\Synth Solo.obj2",
                    ][..]
                } else {
                    &[r"web-render.aux2_p5-midi-test\MIDI Pattern Grid.obj2"][..]
                };
                if class == "Static"
                    && (title.contains(".obj2")
                        || title.contains(".aux2")
                        || title.contains(".dll"))
                {
                    let lines: Vec<_> = title
                        .lines()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .collect();
                    if !lines.is_empty() && lines.iter().all(|line| allowed.contains(line)) {
                        inspection.script = true;
                    } else {
                        inspection.other_script = true;
                    }
                }
                if class == "Button" && title == "このプラグイン・スクリプトを信頼して使用する"
                {
                    inspection.button = window;
                }
            }
        }
        1
    }
    let args: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        args.len() == 2
            || (args.len() == 3
                && matches!(
                    args[2].as_str(),
                    "--trust-p5-script"
                        | "--save-p5-sample"
                        | "--trust-himawari-scripts"
                        | "--save-himawari-sample"
                )),
        "inspect_host_dialog <pid> <expected-development-aviutl2.exe> [--trust-p5-script|--save-p5-sample|--trust-himawari-scripts|--save-himawari-sample]"
    );
    let pid: u32 = args[0].parse()?;
    // SAFETY: read-only process access and correctly sized Win32 structures/buffers.
    unsafe {
        let process = OpenProcess(0x1000, 0, pid);
        anyhow::ensure!(!process.is_null(), "Could not open the development process");
        let mut image = [0u16; 32768];
        let mut length = image.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, image.as_mut_ptr(), &mut length);
        CloseHandle(process);
        anyhow::ensure!(ok != 0, "Could not verify the process executable");
        let actual = std::fs::canonicalize(String::from_utf16_lossy(&image[..length as usize]))?;
        anyhow::ensure!(
            actual == std::fs::canonicalize(&args[1])?,
            "Refused another executable"
        );
        let snapshot = CreateToolhelp32Snapshot(4, 0);
        anyhow::ensure!(snapshot as isize != -1, "Could not enumerate threads");
        let mut entry = ThreadEntry {
            size: size_of::<ThreadEntry>() as u32,
            usage: 0,
            id: 0,
            owner: 0,
            priority: 0,
            delta: 0,
            flags: 0,
        };
        let mut present = Thread32First(snapshot, &mut entry);
        while present != 0 {
            if entry.owner == pid {
                let mut info: GuiInfo = std::mem::zeroed();
                info.size = size_of::<GuiInfo>() as u32;
                if GetGUIThreadInfo(entry.id, &mut info) != 0 && !info.active.is_null() {
                    let mut inspection = Inspection {
                        pid,
                        script: false,
                        other_script: false,
                        button: std::ptr::null_mut(),
                        trust_dialog: false,
                        save_dialog: false,
                        save_question: false,
                        save_button: std::ptr::null_mut(),
                        himawari: args.get(2).is_some_and(|arg| arg.contains("himawari")),
                    };
                    let context = &mut inspection as *mut Inspection as isize;
                    print_window(info.active, context);
                    EnumChildWindows(info.active, print_window, context);
                    if args.get(2).is_some_and(|arg| {
                        arg == "--trust-p5-script" || arg == "--trust-himawari-scripts"
                    }) && inspection.trust_dialog
                        && inspection.script
                        && !inspection.other_script
                        && !inspection.button.is_null()
                    {
                        // BM_CLICK only for the verified host's exact generated test script dialog.
                        anyhow::ensure!(
                            PostMessageW(inspection.button, 0x00f5, 0, 0) != 0,
                            "Could not confirm generated script"
                        );
                        println!("Confirmed only the explicitly allowed generated test scripts");
                    }
                    if args.get(2).is_some_and(|arg| {
                        arg == "--save-p5-sample" || arg == "--save-himawari-sample"
                    }) && inspection.save_dialog
                        && inspection.save_question
                        && !inspection.save_button.is_null()
                    {
                        let owner = GetWindow(info.active, 4);
                        let mut title = [0u16; 1024];
                        let length = GetWindowTextW(owner, title.as_mut_ptr(), title.len() as i32);
                        let title = String::from_utf16_lossy(&title[..length.max(0) as usize]);
                        println!("Save confirmation owner title: {title}");
                        let sample = if inspection.himawari {
                            "himawari-sample.aup2"
                        } else {
                            "p5-midi-sample.aup2"
                        };
                        anyhow::ensure!(
                            title.contains(sample),
                            "Refused a different current project"
                        );
                        let root = actual
                            .parent()
                            .and_then(|p| p.parent())
                            .and_then(|p| p.parent())
                            .unwrap();
                        let saved =
                            std::fs::read_to_string(root.join("examples/aviutl2").join(sample))?;
                        anyhow::ensure!(
                            if inspection.himawari {
                                ["Drum", "Kaiwai Phrase", "Synth Solo"].iter().all(|label| {
                                    saved.contains(&format!("name={label} himawari test"))
                                })
                            } else {
                                saved.contains("name=MIDI Pattern Grid test")
                            },
                            "Saved test backup missing"
                        );
                        anyhow::ensure!(
                            PostMessageW(inspection.save_button, 0x00f5, 0, 0) != 0,
                            "Could not save and close the p5 sample"
                        );
                        println!("Requested normal save and close for {sample}");
                    }
                }
            }
            present = Thread32Next(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
    }
    Ok(())
}
#[cfg(not(windows))]
fn main() {
    eprintln!("Windows-only host diagnostics");
}
