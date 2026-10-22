//! ForskScope desktop entry point.
//!
//! Startup modes:
//!
//! ```
//! forskscope                       # Explorer workspace
//! forskscope <left> <right>        # Two-file diff (git difftool compatible)
//! forskscope <local> <remote> <merged>  # git mergetool: diff local vs remote,
//!                                       # save result to <merged>
//! forskscope compare <left> <right> [options]  # Print a comparison; no window
//! forskscope mcp --root <dir> [--allow-show]   # MCP server for AI agents; no window
//! forskscope --diagnostics         # Print platform diagnostics and exit
//! forskscope --version | -V        # Print the version and exit
//! forskscope --help | -h           # Print this usage and exit
//! forskscope --data-dir <path> ... # Portable: keep everything in <path>
//! forskscope --private ...         # Private: read and keep nothing
//! ```
//!
//! RFC-096 §1: `compare` and `mcp`, exactly as the first argument, hand
//! over to [`forskscope_commands`] before anything below in `main` runs
//! — no GTK, tao, web view, settings, data folder, or Flatpak start-up
//! change (RFC-095 §4.2/§4.7). A file literally named `compare` or `mcp`
//! is written `./compare`/`./mcp`; `git difftool`/`git mergetool` always
//! pass a temporary path first, so they cannot collide with either name.
//!
//! Any other argument count is a startup error (non-zero exit), not a
//! silent fallback to the Explorer workspace (RFC-077). F150 (issue #146): a
//! user looking for the version via the only door CLI convention trains
//! people to try found this error instead - `--version`/`-V` and
//! `--help`/`-h` are handled the same way `--diagnostics` already was,
//! before `parse_startup_args` ever sees them, and the error for every
//! *other* unsupported arity now names `--diagnostics` explicitly, since
//! the old wording led away from the one flag that actually answers "what
//! version is this".
//!
//! `--data-dir`/`--private` (RFC-091) are parsed out of argv the same way
//! `--version` is: before `parse_startup_args` ever sees the file
//! arguments, and before any settings/session/web-view path is resolved —
//! [`startup_mode::DATA_LOCATION`] is set before anything else touches a
//! path. `forskscope --private a.txt b.txt` still opens a comparison; it
//! just starts from defaults and keeps nothing afterward. `--version`,
//! `--help` and `--diagnostics` are still checked first and still exit
//! immediately if present, unchanged — RFC-091 does not combine with any
//! of the three.
//!
//! Exit codes: 1 is a startup-argument error (including an unrecognized
//! `--` flag, and `--data-dir` with no path after it); 2 is either
//! `--private` combined with `--data-dir` (RFC-091 §1: a contradiction, not
//! a priority to resolve by picking one) or, on macOS, `--private` or
//! `--data-dir` used at all (RFC-091 §2a: the system web view there cannot
//! keep either mode's promise, so neither mode starts); 3 (Windows only,
//! F109) is a missing Microsoft Edge WebView2 Runtime, raised whichever
//! button the user presses on the message box offering the download page.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod i18n;
mod keyboard;
mod startup_mode;
mod startup_resolve;
mod state;
#[cfg(test)]
mod test_support;
mod ui;
mod update_check;
mod webview2;

use dioxus_desktop::tao::dpi::LogicalSize;
use dioxus_desktop::{Config, WindowBuilder};
use forskscope_ui_logic::{
    DataLocation, extract_data_location_flags, parse_startup_args, resolve_data_location,
};

use app::{App, STARTUP_REQUEST};
use startup_mode::DATA_LOCATION;

/// Windows-only: the saved language, read without side effects (no
/// migration write, no recovery flow, no file created) via
/// [`forskscope_desktop::persist::schema::settings::SettingsRepository::load`].
/// Falls back to English — matching the owner's 2026-09-17 decision that a
/// first launch is English — whenever no settings file exists yet or it
/// cannot be read as a current/migrated value. RFC-091 §3: private mode
/// reads nothing, so this does not even construct a repository when
/// [`startup_mode::is_private`] — English, the same fallback a missing file
/// already produces.
#[cfg(windows)]
fn saved_lang_no_side_effects() -> state::Lang {
    use forskscope_desktop::persist::schema::PersistenceLoad;
    use forskscope_desktop::persist::schema::settings::SettingsRepository;
    use forskscope_desktop::settings::LocaleId;

    if startup_mode::is_private() {
        return state::Lang::En;
    }

    let repo = SettingsRepository::new(state::config_file_path("settings.json"));
    let language = match repo.load() {
        PersistenceLoad::Current { value } | PersistenceLoad::MigratedLegacy { value, .. } => {
            Some(value.language)
        }
        PersistenceLoad::Missing { .. }
        | PersistenceLoad::FutureVersion { .. }
        | PersistenceLoad::Corrupt { .. } => None,
    };
    match language {
        Some(lang) if lang == LocaleId::japanese() => state::Lang::Ja,
        _ => state::Lang::En,
    }
}

/// Windows-only WebView2 presence check, run after argument parsing and
/// before the window is created (F109). Wires the OS calls to
/// [`webview2::decide`]: reads the runtime version, on failure prints to
/// stderr, shows a message box, and on "Yes" opens the download page via
/// `cmd /C start "" <url>` — the standard way to open a URL from a Windows
/// console app with no new dependency; passing the empty string as the
/// explicit window-title argument avoids `start` misreading the URL itself
/// as a quoted title, which is the usual quoting pitfall with this command.
#[cfg(windows)]
fn check_webview2_or_exit() {
    let version = wry::webview_version().map_err(|e| e.to_string());
    let lang = saved_lang_no_side_effects();
    if let webview2::Decision::Stop {
        stderr_line,
        dialog_text,
    } = webview2::decide(version, lang)
    {
        eprintln!("{stderr_line}");
        let pressed_yes = rfd::MessageDialog::new()
            .set_title("ForskScope")
            .set_level(rfd::MessageLevel::Error)
            .set_buttons(rfd::MessageButtons::YesNo)
            .set_description(dialog_text)
            .show()
            == rfd::MessageDialogResult::Yes;
        if pressed_yes {
            let mut cmd = std::process::Command::new("cmd");
            cmd.args([
                "/C",
                "start",
                "",
                "https://developer.microsoft.com/microsoft-edge/webview2/",
            ]);
            // RFC-091 §2: the browser must see the user's own environment,
            // not any XDG_* override portable/private mode made — a no-op
            // in Installed mode, where nothing was ever overridden. (Windows
            // does not read the XDG_* variables itself, but this process
            // still should not hand an overridden value to a child that
            // outlives it.)
            forskscope_desktop::env_passthrough::restore_onto(&mut cmd);
            let _ = cmd.spawn();
        }
        std::process::exit(3);
    }
}

/// RFC-096 §4: a subcommand's own output must reach the console it was
/// launched from, but the app is a windowed program
/// (`windows_subsystem = "windows"`), which starts with no console of
/// its own. Piped use — how MCP clients start a subcommand, and the
/// shape `git difftool`/`git mergetool` themselves use for the app's
/// own mode — must change nothing. `GetFileType` on the standard output
/// handle tells a real console apart from a redirected one: a disk file
/// (`FILE_TYPE_DISK`) or a pipe (`FILE_TYPE_PIPE`) means "already
/// redirected, leave it exactly as the parent set it up"; anything else
/// (a real console, or no handle at all) means "attach, so the parent's
/// own console shows this process's output." A failed `AttachConsole`
/// (no parent console to attach to — launched with no terminal at all)
/// is left unhandled on purpose: there is nothing to attach to, and the
/// subcommand still runs exactly as it would without this function
/// existing — its output simply has nowhere to be seen.
#[cfg(windows)]
fn attach_parent_console_if_not_redirected() {
    use windows_sys::Win32::Storage::FileSystem::{FILE_TYPE_DISK, FILE_TYPE_PIPE, GetFileType};
    use windows_sys::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_OUTPUT_HANDLE,
    };

    // SAFETY: GetStdHandle/GetFileType/AttachConsole are plain Win32
    // calls with no preconditions beyond a valid process — every one of
    // these functions already tolerates an invalid/absent handle by
    // design (that is exactly the "no console at all" case this
    // function leaves alone).
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let file_type = GetFileType(handle);
        if file_type != FILE_TYPE_DISK && file_type != FILE_TYPE_PIPE {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

/// RFC-095 §4.2/§4.7: the three Flatpak-only start-up changes, in the
/// order the RFC gives them, each a no-op outside a Flatpak
/// ([`forskscope_desktop::platform::flatpak`] — every Flatpak behaviour
/// calls that, never checks `/.flatpak-info` itself). Called once, as
/// the very first thing in `main` after RFC-096 §1's subcommand
/// dispatch returns — see the call site's own comment for why that
/// still counts as "before any thread starts."
#[cfg(target_os = "linux")]
fn apply_flatpak_startup_changes() {
    let flatpak = forskscope_desktop::platform::flatpak();
    if !flatpak.in_flatpak {
        return;
    }

    // §4.7 point 1: the program name becomes the Flatpak app ID, never a
    // GTK application ID — `tao` registers a unique `GApplication` when
    // given one, which would make the app single-instance; a second
    // `git difftool` launch would then hand over to the already-running
    // first one instead of opening its own window. `glib::set_prgname`
    // must run before GTK initialises, which nothing above this point
    // has done.
    if let Some(app_id) = &flatpak.app_id {
        glib::set_prgname(Some(app_id.as_str()));
    }

    // §4.2: WebKit's network process asks GLib for a proxy before it
    // connects anywhere — including to Dioxus's own 127.0.0.1 WebSocket,
    // which delivers the interface inside the sandbox. Without network
    // permission, the proxy portal's answer is "no route at all"
    // (`Address enumeration completed (out of addresses)`), and the
    // window stays blank. `dummy` means direct connections, which loses
    // nothing here: the web view only ever loads the app's own content.
    // The user's own value, if set, always wins.
    if std::env::var_os("GIO_USE_PROXY_RESOLVER").is_none() {
        // SAFETY: this is the first thing `main` does after RFC-096's
        // subcommand dispatch returns — nothing before this point has
        // spawned a thread, so no other thread can be reading or
        // writing the process environment concurrently.
        unsafe {
            std::env::set_var("GIO_USE_PROXY_RESOLVER", "dummy");
        }
    }

    // §4.7 point 2: WebKitGTK's web-process sandbox is opt-in for the
    // GTK 3 API (`webkit2gtk-4.1`) `wry` uses here, and only inside that
    // sandbox can the web process own a name on the accessibility bus
    // (RFC-095 §2's stage-1 result) — without it, a screen-reader user
    // meets an empty window. `wry`'s own public API exposes no
    // `WebContext` before the first web view exists (its internal
    // `WebContextImpl` field is private), so
    // `webkit2gtk::WebContext::set_sandbox_enabled(true)` is unreachable
    // from this codebase today; `WEBKIT_FORCE_SANDBOX=1` — the same
    // variable stage 1's own test build used — is the one lever actually
    // available. The user's own value, if set, always wins.
    if std::env::var_os("WEBKIT_FORCE_SANDBOX").is_none() {
        // SAFETY: see above — still before any other thread exists.
        unsafe {
            std::env::set_var("WEBKIT_FORCE_SANDBOX", "1");
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // RFC-096 §1: the first argument decides, before anything else in
    // `main` — see this module's own doc comment for what "before
    // anything else" guarantees.
    match args.first().map(String::as_str) {
        Some("compare") => {
            #[cfg(windows)]
            attach_parent_console_if_not_redirected();
            std::process::exit(forskscope_commands::compare::run(&args[1..]));
        }
        Some("mcp") => {
            #[cfg(windows)]
            attach_parent_console_if_not_redirected();
            std::process::exit(forskscope_commands::mcp::run(&args[1..]));
        }
        _ => {}
    }

    // RFC-095 §4.2/§4.7: Flatpak-only start-up changes, before anything
    // else below — the next line after this one is the first to read an
    // argument flag or touch a path, and nothing above this point (the
    // subcommand dispatch, just returned from above) spawns a thread or
    // touches GTK/GLib/WebKit, so this is still "before any thread
    // starts," as the RFC asks. A no-op outside a Flatpak.
    #[cfg(target_os = "linux")]
    apply_flatpak_startup_changes();

    // F150: `--version`/`-V` and `--help`/`-h`, handled the same way
    // `--diagnostics` already is - before `parse_startup_args` ever sees
    // them, so neither competes with the 0/2/3-argument modes it parses.
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("forskscope {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!(
            "Usage:\n  \
             forskscope                              Open the Explorer workspace\n  \
             forskscope <left> <right>                Compare two files (git difftool compatible)\n  \
             forskscope <local> <remote> <merged>     git mergetool: diff local vs remote, save to merged\n  \
             forskscope compare <left> <right> [opts] Print a comparison; no window (see forskscope compare --help)\n  \
             forskscope mcp --root <dir> [--allow-show]  MCP server for AI agents; no window (see forskscope mcp --help)\n  \
             forskscope --diagnostics                 Print platform diagnostics and exit\n  \
             forskscope --version, -V                 Print the version and exit\n  \
             forskscope --help, -h                    Print this message and exit\n"
        );
        // RFC-091 §2a: neither mode can keep its promise on macOS (the
        // system web view there keeps its own cache regardless), so both
        // refuse to start — listing them here, even with a caveat, would
        // describe something that only exits 2. Omitted, not qualified.
        #[cfg(not(target_os = "macos"))]
        print!(
            "  \
             forskscope --data-dir <path> ...         Portable: keep settings, session and\n  \
             \x20                                        everything else this writes inside <path>\n  \
             \x20                                        instead of the usual system locations\n  \
             forskscope --private ...                 Private: start with default settings, and\n  \
             \x20                                        save nothing — the files you compare and\n  \
             \x20                                        save are still your files, written when\n  \
             \x20                                        you save\n"
        );
        return;
    }

    // --diagnostics: print platform info and exit without launching the UI.
    // Useful for debugging startup failures and filing bug reports.
    if args.iter().any(|a| a == "--diagnostics") {
        let info = forskscope_desktop::platform::PlatformInfo::collect();
        println!("{}", info.to_report());
        #[cfg(windows)]
        match wry::webview_version() {
            Ok(version) => println!("WebView2 runtime: {version}"),
            Err(error) => println!("WebView2 runtime: not found ({error})"),
        }
        return;
    }

    // RFC-091 §1/§5: --private and --data-dir, parsed the same way
    // --version already is — before parse_startup_args ever sees the file
    // arguments, and before any settings/session/web-view path is
    // resolved. An unrecognized `--` flag is rejected here rather than
    // falling through to parse_startup_args as a literal file name, which
    // would fail later with a confusing "file not found" instead of
    // naming the actual mistake.
    let (flags, remaining_args) = match extract_data_location_flags(&args) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("forskscope: {error}");
            std::process::exit(1);
        }
    };
    let program_dir = startup_resolve::program_dir();
    let marker_exists = program_dir
        .as_deref()
        .is_some_and(startup_resolve::marker_exists);
    let data_location = match resolve_data_location(
        startup_resolve::current_platform(),
        flags.private,
        flags.data_dir,
        program_dir.as_deref(),
        marker_exists,
    ) {
        Ok(location) => location,
        Err(error) => {
            eprintln!("forskscope: {error}");
            std::process::exit(2);
        }
    };
    if let DataLocation::Portable(dir) = &data_location {
        // RFC-091 §1: "--data-dir's folder is created if it does not
        // exist." The marker-folder case already exists by definition
        // (that is how it was found); create_dir_all is a no-op on it.
        //
        // RFC-091 §2's "Read-only data location" criterion: a folder that
        // cannot be created, or exists but fails the write probe, is not a
        // reason to exit — ForskScope still starts, reads what it can from
        // it, and saves nothing (startup_mode::is_portable_read_only, which
        // every settings/session/manifest write site checks). Detected by
        // an actual write attempt, not permission bits, which can lie.
        let writable = std::fs::create_dir_all(dir).is_ok() && startup_resolve::probe_writable(dir);
        let _ = startup_mode::PORTABLE_WRITABLE.set(writable);
    }
    // Set before anything else touches a settings/session/web-view path —
    // config_file_path, the manifest folder and the web-view Config below
    // all derive from this one value (RFC-091 §1: "The decision is made
    // once").
    let _ = DATA_LOCATION.set(data_location.clone());

    // RFC-091 §2/review 167: redirect WebKitGTK/GLib/GTK's own cache, data
    // and state locations via the XDG Base Directory variables, before GTK
    // (or any other thread) can read them — see
    // `startup_resolve::redirect_xdg_dirs`'s doc comment for why
    // `with_data_directory` alone is not enough on Linux. Must run this
    // early: nothing above this line touches GTK or spawns a thread.
    #[cfg(target_os = "linux")]
    startup_resolve::redirect_xdg_dirs(&data_location);

    match parse_startup_args(&remaining_args) {
        Ok(request) => {
            let _ = STARTUP_REQUEST.set(request);
        }
        Err(error) => {
            eprintln!("forskscope: {error}");
            std::process::exit(1);
        }
    }

    #[cfg(windows)]
    check_webview2_or_exit();

    let window = WindowBuilder::new()
        .with_title("ForskScope")
        .with_inner_size(LogicalSize::new(1180.0, 760.0));

    let config = startup_resolve::configure_webview_data_dir(
        Config::new().with_window(window).with_menu(None),
        &data_location,
    );

    dioxus_desktop::launch::launch(App, Vec::new(), vec![Box::new(config)]);
}
