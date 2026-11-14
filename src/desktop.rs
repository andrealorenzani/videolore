// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Desktop integration.
//!
//! - Linux: the AppImage and the tar.gz have no installer, so at start-up the app adds its
//!   own application-menu entry, icon and `.vlproj` file type under `~/.local/share`, and
//!   refreshes them when the file was moved. Source builds (`cargo run`) do nothing. The
//!   About screen can remove the entry again.
//! - macOS: a double-clicked project reaches the running app as an Apple Event (`odoc`),
//!   which winit does not expose; [`macos::install`] registers a handler for it.
//! - Every OS: projects to open arrive through [`push_open_request`] and are collected by
//!   the app with [`take_open_requests`] (argv at start-up, Apple Events later).

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// Marker file shipped in the Linux tar.gz next to the executable.
pub const BUNDLE_MARKER: &str = ".videolore-bundle";
/// MIME type of project files.
pub const MIME_TYPE: &str = "application/x-videolore-project";
/// Icon written for the menu entry (256x256 PNG).
pub const ICON_PNG: &[u8] = include_bytes!("../assets/icon/icon-256.png");

const DESKTOP_FILE: &str = "applications/videolore.desktop";
const ICON_FILE: &str = "icons/hicolor/256x256/apps/videolore.png";
const MIME_FILE: &str = "mime/packages/videolore.xml";

static OPEN_REQUESTS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Queues a project to open (from argv or the OS).
pub fn push_open_request(path: PathBuf) {
    if let Ok(mut q) = OPEN_REQUESTS.lock() {
        q.push(path);
    }
}

/// Takes the queued projects to open.
pub fn take_open_requests() -> Vec<PathBuf> {
    OPEN_REQUESTS
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default()
}

/// The project given on the command line, if any. macOS adds `-psn_…` when launched from
/// the Finder; it is ignored.
pub fn project_arg<I: IntoIterator<Item = std::ffi::OsString>>(args: I) -> Option<PathBuf> {
    args.into_iter()
        .skip(1)
        .find(|a| !a.to_string_lossy().starts_with("-psn_"))
        .map(PathBuf::from)
}

/// The program the menu entry must start, or `None` when this copy is not an installed
/// AppImage or tar.gz bundle (source builds are left alone).
pub fn launch_target(appimage: Option<&OsStr>, exe: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = appimage.map(PathBuf::from)
        && p.is_absolute()
        && p.is_file()
    {
        return Some(p);
    }
    let exe = exe?;
    let dir = exe.parent()?;
    if dir.join(BUNDLE_MARKER).is_file() {
        return Some(std::fs::canonicalize(exe).unwrap_or_else(|_| exe.to_path_buf()));
    }
    None
}

/// Quotes a program path for the `Exec` key of a desktop entry (Desktop Entry spec:
/// quoted argument, then the string escaping of the file format).
pub fn quote_exec(path: &Path) -> String {
    let mut quoted = String::from("\"");
    for c in path.to_string_lossy().chars() {
        match c {
            '"' | '`' | '$' => {
                quoted.push('\\');
                quoted.push(c);
            }
            // a backslash is escaped once for the quoting and once for the string value
            '\\' => quoted.push_str("\\\\\\\\"),
            '%' => quoted.push_str("%%"),
            '\n' | '\r' => quoted.push(' '),
            c => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

/// The `.desktop` file for `target`.
pub fn desktop_entry(target: &Path) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=videolore\n\
         GenericName=Video editor\n\
         Comment=Edit short videos for social media\n\
         Exec={} %f\n\
         Icon=videolore\n\
         Terminal=false\n\
         Categories=AudioVideo;Video;AudioVideoEditing;\n\
         MimeType={MIME_TYPE};\n\
         StartupWMClass=videolore\n",
        quote_exec(target)
    )
}

/// The shared-mime-info package declaring `*.vlproj`.
pub fn mime_xml() -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <mime-info xmlns=\"http://www.freedesktop.org/standards/shared-mime-info\">\n\
         \x20 <mime-type type=\"{MIME_TYPE}\">\n\
         \x20   <comment>videolore project</comment>\n\
         \x20   <glob pattern=\"*.vlproj\"/>\n\
         \x20   <icon name=\"videolore\"/>\n\
         \x20 </mime-type>\n\
         </mime-info>\n"
    )
}

/// The files the integration owns under the user's data folder (`~/.local/share`).
pub fn owned_files(data_dir: &Path) -> [PathBuf; 3] {
    [
        data_dir.join(DESKTOP_FILE),
        data_dir.join(ICON_FILE),
        data_dir.join(MIME_FILE),
    ]
}

/// Writes the menu entry, icon and file type. Returns `true` when something changed (a new
/// install, or the program moved); an up-to-date integration is left untouched.
pub fn install(data_dir: &Path, target: &Path, icon_png: &[u8]) -> io::Result<bool> {
    let [desktop, icon, mime] = owned_files(data_dir);
    let wanted: [(&Path, Vec<u8>); 3] = [
        (&desktop, desktop_entry(target).into_bytes()),
        (&icon, icon_png.to_vec()),
        (&mime, mime_xml().into_bytes()),
    ];
    let mut changed = false;
    for (path, bytes) in wanted {
        if std::fs::read(path).is_ok_and(|old| old == bytes) {
            continue;
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, bytes)?;
        changed = true;
    }
    Ok(changed)
}

/// Removes the menu entry, icon and file type (the About screen's "Remove from application
/// menu"). Projects are never touched.
pub fn remove(data_dir: &Path) -> io::Result<()> {
    for f in owned_files(data_dir) {
        match std::fs::remove_file(&f) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
    }
    Ok(())
}

/// Asks the desktop to re-read the menu entries and file types. Best effort: the helper
/// programs may be missing; no shell is involved.
pub fn refresh_databases(data_dir: &Path) {
    for (prog, sub) in [
        ("update-mime-database", "mime"),
        ("update-desktop-database", "applications"),
    ] {
        let _ = Command::new(prog)
            .arg(data_dir.join(sub))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// The user's data folder (`$XDG_DATA_HOME` or `~/.local/share`).
pub fn user_data_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|b| b.data_dir().to_path_buf())
}

/// Start-up self-integration on Linux; does nothing elsewhere or for source builds.
pub fn integrate_at_startup() {
    if !cfg!(target_os = "linux") {
        return;
    }
    let appimage = std::env::var_os("APPIMAGE");
    let exe = std::env::current_exe().ok();
    if let (Some(target), Some(data)) = (
        launch_target(appimage.as_deref(), exe.as_deref()),
        user_data_dir(),
    ) && install(&data, &target, ICON_PNG).unwrap_or(false)
    {
        refresh_databases(&data);
    }
}

/// Whether this copy integrates itself in the menu (and so can remove itself from it).
pub fn self_integrating() -> bool {
    cfg!(target_os = "linux")
        && launch_target(
            std::env::var_os("APPIMAGE").as_deref(),
            std::env::current_exe().ok().as_deref(),
        )
        .is_some()
}

/// Removes the Linux menu entry of the current user.
pub fn remove_integration() -> io::Result<()> {
    let data = user_data_dir().ok_or_else(|| io::Error::other("no home folder"))?;
    remove(&data)?;
    refresh_databases(&data);
    Ok(())
}

#[cfg(target_os = "macos")]
pub mod macos {
    //! Apple Event `odoc` handler: Finder sends the double-clicked project to the running
    //! app. The handler is registered when the application is about to finish launching
    //! (so a project opened at launch is caught and AppKit's default handler is replaced),
    //! and every received path goes to [`super::push_open_request`].
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
    use objc2::{AllocAnyThread, define_class, msg_send, sel};
    use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventManager, NSNotification, NSString};

    /// `kCoreEventClass` ('aevt').
    const CORE_EVENT_CLASS: u32 = u32::from_be_bytes(*b"aevt");
    /// `kAEOpenDocuments` ('odoc').
    const OPEN_DOCUMENTS: u32 = u32::from_be_bytes(*b"odoc");
    /// `keyDirectObject` ('----').
    const DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "VideoloreOpenDocuments"]
        struct OpenHandler;

        unsafe impl NSObjectProtocol for OpenHandler {}

        impl OpenHandler {
            #[unsafe(method(willFinishLaunching:))]
            fn will_finish_launching(&self, _note: &NSNotification) {
                let manager = NSAppleEventManager::sharedAppleEventManager();
                // SAFETY: `self` is a live Objective-C object that responds to the selector
                // `handleOpen:reply:` (defined below, signature (id, id) -> void, which is
                // what NSAppleEventManager calls); the two codes are u32 FourCharCodes as
                // the method expects. The handler object is leaked in `install`, so it
                // outlives the registration.
                unsafe {
                    let _: () = msg_send![
                        &*manager,
                        setEventHandler: self as &AnyObject,
                        andSelector: sel!(handleOpen:reply:),
                        forEventClass: CORE_EVENT_CLASS,
                        andEventID: OPEN_DOCUMENTS
                    ];
                }
            }

            #[unsafe(method(handleOpen:reply:))]
            fn handle_open(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
                // SAFETY: `paramDescriptorForKeyword:` takes an AEKeyword (u32) and returns
                // a possibly-nil NSAppleEventDescriptor; types match the Objective-C method.
                let list: Option<Retained<NSAppleEventDescriptor>> =
                    unsafe { msg_send![event, paramDescriptorForKeyword: DIRECT_OBJECT] };
                let Some(list) = list else { return };
                // AppleEvent lists are 1-based; a single file may arrive as a non-list.
                let n = list.numberOfItems().max(1);
                for i in 1..=n {
                    let item = if list.numberOfItems() == 0 {
                        Some(list.clone())
                    } else {
                        list.descriptorAtIndex(i)
                    };
                    if let Some(path) = item
                        .and_then(|d| d.fileURLValue())
                        .and_then(|url| url.path())
                    {
                        super::push_open_request(path.to_string().into());
                    }
                }
            }
        }
    );

    /// Registers the handler. Call from `main` before the event loop starts.
    pub fn install() {
        // SAFETY: plain `init` of an NSObject subclass without instance variables.
        let handler: Retained<OpenHandler> =
            unsafe { msg_send![super(OpenHandler::alloc().set_ivars(())), init] };
        let center = objc2_foundation::NSNotificationCenter::defaultCenter();
        let name = NSString::from_str("NSApplicationWillFinishLaunchingNotification");
        // SAFETY: the observer responds to `willFinishLaunching:` taking one NSNotification;
        // it is leaked below so the notification center never sees a dangling observer.
        unsafe {
            center.addObserver_selector_name_object(
                &handler,
                sel!(willFinishLaunching:),
                Some(&name),
                None,
            );
        }
        // The handler must live as long as the app (AppKit keeps unretained references).
        std::mem::forget(handler);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-041
    #[test]
    fn menu_entry_and_file_type_content() {
        let e = desktop_entry(Path::new("/home/a b/Apps/videolore-0.2.0.AppImage"));
        assert!(e.starts_with("[Desktop Entry]\n"));
        assert!(e.contains("Exec=\"/home/a b/Apps/videolore-0.2.0.AppImage\" %f\n"));
        assert!(e.contains("MimeType=application/x-videolore-project;\n"));
        assert!(e.contains("Icon=videolore\n"));
        let x = mime_xml();
        assert!(x.contains("<glob pattern=\"*.vlproj\"/>"));
        assert!(x.contains("type=\"application/x-videolore-project\""));
        // special characters are quoted per the Desktop Entry spec
        assert_eq!(quote_exec(Path::new("/a\"b$c`d")), "\"/a\\\"b\\$c\\`d\"");
        assert_eq!(quote_exec(Path::new("/x\\y%z")), "\"/x\\\\\\\\y%%z\"");
        assert_eq!(quote_exec(Path::new("/x\ny")), "\"/x y\"");
        assert_eq!(&ICON_PNG[1..4], b"PNG");
    }

    // covers: REQ-041
    #[test]
    fn appimage_and_bundle_are_detected_source_builds_are_not() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("videolore.AppImage");
        std::fs::write(&app, "").unwrap();
        let exe = dir.path().join("bundle/videolore");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, "").unwrap();
        // AppImage wins when $APPIMAGE points to a real file
        assert_eq!(launch_target(Some(app.as_os_str()), Some(&exe)), Some(app));
        // a cargo build: no marker, nothing to do
        assert_eq!(launch_target(None, Some(&exe)), None);
        assert_eq!(
            launch_target(Some(OsStr::new("relative")), Some(&exe)),
            None
        );
        assert_eq!(launch_target(None, None), None);
        // the tar.gz ships a marker next to the executable
        std::fs::write(dir.path().join("bundle").join(BUNDLE_MARKER), "").unwrap();
        let t = launch_target(None, Some(&exe)).unwrap();
        assert!(t.ends_with("bundle/videolore"));
    }

    // covers: REQ-041
    #[test]
    fn install_is_idempotent_refreshes_on_move_and_removes() {
        let data = tempfile::tempdir().unwrap();
        let projects = data.path().join("my.vlproj");
        std::fs::write(&projects, "{}").unwrap();
        assert!(install(data.path(), Path::new("/opt/a/videolore"), ICON_PNG).unwrap());
        for f in owned_files(data.path()) {
            assert!(f.is_file(), "{}", f.display());
        }
        // same target again: nothing rewritten
        assert!(!install(data.path(), Path::new("/opt/a/videolore"), ICON_PNG).unwrap());
        // the AppImage moved: the entry follows it
        assert!(install(data.path(), Path::new("/opt/b/videolore"), ICON_PNG).unwrap());
        let entry = std::fs::read_to_string(&owned_files(data.path())[0]).unwrap();
        assert!(entry.contains("Exec=\"/opt/b/videolore\" %f"));
        refresh_databases(data.path());
        remove(data.path()).unwrap();
        remove(data.path()).unwrap();
        assert!(owned_files(data.path()).iter().all(|f| !f.exists()));
        // projects are never touched
        assert!(projects.is_file());
        // a read-only location reports the error
        let file = data.path().join("plain-file");
        std::fs::write(&file, "").unwrap();
        assert!(install(&file, Path::new("/x"), ICON_PNG).is_err());
    }

    // covers: REQ-041
    #[test]
    fn projects_to_open_come_from_argv_and_the_os() {
        let args = |v: &[&str]| v.iter().map(std::ffi::OsString::from).collect::<Vec<_>>();
        assert_eq!(
            project_arg(args(&["videolore", "/p/a.vlproj"])),
            Some(PathBuf::from("/p/a.vlproj"))
        );
        assert_eq!(
            project_arg(args(&["videolore", "-psn_0_12345", "/p/b.vlproj"])),
            Some(PathBuf::from("/p/b.vlproj"))
        );
        assert_eq!(project_arg(args(&["videolore"])), None);
        push_open_request("/p/c.vlproj".into());
        assert!(take_open_requests().contains(&PathBuf::from("/p/c.vlproj")));
        // the start-up hooks are safe in a source build
        integrate_at_startup();
        let _ = self_integrating();
        assert!(user_data_dir().is_some());
    }
}
