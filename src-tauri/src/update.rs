//! Updates of Fox Git itself, from the author's site, by the scheme of Wine Fox
//! (`packaging/SELF-UPDATE.md` there).
//!
//! The site keeps one manifest per kind of build next to the files:
//!
//! ```text
//! https://downloads.lis-carter.site/fox-git/latest-setup.json     Windows installer
//! https://downloads.lis-carter.site/fox-git/latest-portable.json  Windows portable exe
//! https://downloads.lis-carter.site/fox-git/latest-deb.json       Linux .deb
//! {"version": "0.81.0", "file": "...", "url": "https://...", "size": 123, "sha256": "..."}
//! ```
//!
//! `scripts/latest.mjs` writes them after a build. A running exe cannot be overwritten on
//! Windows, so the portable one is renamed to `fox-git.exe.old` and the new file takes its
//! name; the installed one runs the new installer after the app closes.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

const BASE: &str = "https://downloads.lis-carter.site/fox-git/";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const MANIFEST_LIMIT: u64 = 1 << 16;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Setup,
    Portable,
    Deb,
}

impl Kind {
    fn manifest(self) -> &'static str {
        match self {
            Kind::Setup => "latest-setup.json",
            Kind::Portable => "latest-portable.json",
            Kind::Deb => "latest-deb.json",
        }
    }
}

/// How this copy was installed. Empty when it runs from the sources, which updates cannot replace.
fn kind() -> Option<Kind> {
    let exe = std::env::current_exe().ok()?;
    if cfg!(debug_assertions) {
        return None;
    }
    if cfg!(windows) {
        // The NSIS installer puts its uninstaller next to the app.
        Some(if exe.with_file_name("uninstall.exe").is_file() { Kind::Setup } else { Kind::Portable })
    } else {
        exe.starts_with("/usr/").then_some(Kind::Deb)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Update {
    version: String,
    file: String,
    url: String,
    size: u64,
    sha256: String,
}

/// What the UI shows about a found update.
#[derive(Serialize)]
pub struct Found {
    version: String,
    /// False when this copy cannot replace itself: the UI sends the user to the site.
    installable: bool,
}

/// The update between the steps: found, then downloaded, then started on restart or exit.
#[derive(Default)]
pub struct Pending {
    found: Mutex<Option<Update>>,
    /// What to start: the new installer, or the replaced app.
    ready: Mutex<Option<Ready>>,
}

enum Ready {
    /// Runs after the app closes; it replaces the files and can start the new version.
    Installer(PathBuf),
    /// The app's own file, already replaced.
    App(PathBuf),
}

/// `0.10` is newer than `0.9`: numbers compare as numbers.
fn key(version: &str) -> Vec<(u8, u64, String)> {
    version
        .split(['.', '-'])
        .map(|part| match part.parse() {
            Ok(n) => (0, n, String::new()),
            Err(_) => (1, 0, part.to_string()),
        })
        .collect()
}

fn newer(version: &str, than: &str) -> bool {
    key(version) > key(than)
}

/// Reads `latest-*.json` and refuses anything but an HTTPS file with a full SHA-256.
fn parse(raw: &[u8]) -> Result<Update, String> {
    let mut update: Update = serde_json::from_slice(raw).map_err(|e| e.to_string())?;
    update.sha256.make_ascii_lowercase();
    let hex = update.sha256.len() == 64 && update.sha256.bytes().all(|b| b.is_ascii_hexdigit());
    if !update.url.starts_with("https://") || !hex || update.version.is_empty() {
        return Err("latest.json".into());
    }
    Ok(update)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(60))
        .build()
}

fn manifest_url(kind: Kind) -> String {
    // For tests: another folder with the manifests.
    let base = std::env::var("FOXGIT_UPDATE_BASE").unwrap_or_else(|_| BASE.into());
    format!("{}/{}", base.trim_end_matches('/'), kind.manifest())
}

/// The update the site offers; empty when this is the newest version.
#[tauri::command(async)]
pub fn update_check(pending: State<'_, Pending>) -> Result<Option<Found>, String> {
    let checked = kind().unwrap_or(if cfg!(windows) { Kind::Setup } else { Kind::Deb });
    let mut raw = vec![];
    agent()
        .get(&manifest_url(checked))
        .call()
        .map_err(|e| e.to_string())?
        .into_reader()
        .take(MANIFEST_LIMIT)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    let update = parse(&raw)?;
    if !newer(&update.version, VERSION) {
        return Ok(None);
    }
    let found = Found { version: update.version.clone(), installable: kind().is_some() };
    *pending.found.lock().unwrap() = Some(update);
    Ok(Some(found))
}

/// Copies the download into `part`, checking its size and SHA-256. On any failure `part` is gone.
fn save(mut body: impl Read, update: &Update, part: &Path, progress: &dyn Fn(u64)) -> Result<(), String> {
    let result = (|| {
        let mut out = std::fs::File::create(part).map_err(|e| e.to_string())?;
        let mut digest = Sha256::new();
        let mut done = 0u64;
        let mut buffer = vec![0; 1 << 20];
        loop {
            let n = body.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            std::io::Write::write_all(&mut out, &buffer[..n]).map_err(|e| e.to_string())?;
            digest.update(&buffer[..n]);
            done += n as u64;
            if done > update.size {
                break;
            }
            progress(done);
        }
        let sha: String = digest.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if done != update.size || sha != update.sha256 {
            return Err("The downloaded file is damaged (size or checksum), so the update was not installed".into());
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(part);
    }
    result
}

/// Puts `part` in place of the running exe under the same name; the old one becomes `.old`
/// and goes away on the next start. Nothing changes when a step fails.
fn replace_running(part: &Path, target: &Path) -> Result<(), String> {
    let old = old_copy(target);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(target, &old).map_err(|e| e.to_string())?;
    if let Err(e) = std::fs::rename(part, target) {
        let _ = std::fs::rename(&old, target);
        return Err(e.to_string());
    }
    Ok(())
}

fn old_copy(target: &Path) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".old");
    target.with_file_name(name)
}

fn update_dir() -> PathBuf {
    std::env::temp_dir().join("fox-git-update")
}

/// Removes what the last update left: the old portable exe and downloaded packages.
pub fn clean_up() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(old_copy(&exe));
    }
    let _ = std::fs::remove_dir_all(update_dir());
}

/// Downloads the found update and installs it as far as the running app allows.
/// Sends the download progress in percent.
#[tauri::command(async)]
pub fn update_install(pending: State<'_, Pending>, progress: Channel<u32>) -> Result<(), String> {
    let update = pending.found.lock().unwrap().clone().ok_or("There is no update to install")?;
    let kind = kind().ok_or("This copy cannot update itself, download the new version from the site")?;
    let response = agent().get(&update.url).call().map_err(|e| e.to_string())?;
    let report = |done: u64| {
        let _ = progress.send((done * 100 / update.size.max(1)) as u32);
    };
    let fail = |e: String| format!("Could not install the update: {e}");

    let ready = match kind {
        Kind::Portable => {
            let target = std::env::current_exe().map_err(|e| e.to_string())?;
            let name = target.file_name().unwrap_or_default().to_string_lossy().to_string();
            let part = target.with_file_name(format!(".{name}.part"));
            save(response.into_reader(), &update, &part, &report).map_err(fail)?;
            replace_running(&part, &target).map_err(fail)?;
            Ready::App(target)
        }
        Kind::Setup | Kind::Deb => {
            let dir = update_dir();
            std::fs::create_dir_all(&dir).map_err(|e| fail(e.to_string()))?;
            let file = dir.join(Path::new(&update.file).file_name().ok_or("latest.json")?);
            let part = dir.join(format!(".{}.part", file.file_name().unwrap().to_string_lossy()));
            save(response.into_reader(), &update, &part, &report).map_err(fail)?;
            std::fs::rename(&part, &file).map_err(|e| fail(e.to_string()))?;
            if kind == Kind::Setup {
                Ready::Installer(file)
            } else {
                // The system asks for the password; apt replaces the files of the running app.
                let status = Command::new("pkexec")
                    .args(["apt-get", "install", "-y"])
                    .arg(&file)
                    .status()
                    .map_err(|e| fail(e.to_string()))?;
                let _ = std::fs::remove_file(&file);
                if !status.success() {
                    return Err(fail(format!("pkexec apt-get exited with {status}")));
                }
                Ready::App(std::env::current_exe().map_err(|e| e.to_string())?)
            }
        }
    };
    *pending.ready.lock().unwrap() = Some(ready);
    Ok(())
}

fn start(ready: &Ready, restart: bool) -> std::io::Result<()> {
    let mut command = match ready {
        Ready::Installer(path) => {
            let mut command = Command::new(path);
            command.args(["/P", "/UPDATE"]);
            if restart {
                command.arg("/R");
            }
            command
        }
        Ready::App(path) => Command::new(path),
    };
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map(|_| ())
}

/// Starts the new version and closes this one.
#[tauri::command]
pub fn update_restart(app: AppHandle, pending: State<'_, Pending>) -> Result<(), String> {
    let ready = pending.ready.lock().unwrap().take().ok_or("There is no update to start")?;
    start(&ready, true).map_err(|e| e.to_string())?;
    app.exit(0);
    Ok(())
}

/// An installer the user put off still runs when the app closes, without starting it again.
pub fn on_exit(pending: State<'_, Pending>) {
    if let Some(ready @ Ready::Installer(_)) = pending.ready.lock().unwrap().take() {
        let _ = start(&ready, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(body: &[u8]) -> Update {
        let sha: String = Sha256::digest(body).iter().map(|b| format!("{b:02x}")).collect();
        Update { version: "9.0".into(), file: "f".into(), url: "https://x/f".into(), size: body.len() as u64, sha256: sha }
    }

    #[test]
    fn versions_compare_as_numbers() {
        assert!(newer("0.10", "0.9"));
        assert!(newer("0.81.0", "0.8.0"));
        assert!(newer("0.90.0", "0.89.0"));
        assert!(!newer("0.8.0", "0.8.0"));
        assert!(!newer("0.8.0", "0.81.0"));
    }

    #[test]
    fn broken_manifests_are_refused() {
        let good = r#"{"version":"0.81.0","file":"a.exe","url":"https://downloads.lis-carter.site/fox-git/a.exe","size":5,"sha256":"ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef0123456789"}"#;
        assert_eq!(parse(good.as_bytes()).unwrap().sha256.len(), 64);
        assert!(parse(b"<html>").is_err());
        assert!(parse(good.replace("https://", "http://").as_bytes()).is_err());
        assert!(parse(good.replace("ABCDEF", "XYZ").as_bytes()).is_err());
        assert!(parse(good.replace("\"size\":5", "\"size\":\"5\"").as_bytes()).is_err());
    }

    #[test]
    fn a_damaged_download_leaves_the_old_file() {
        let dir = std::env::temp_dir().join("fox-git-update-test-bad");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (target, part) = (dir.join("fox-git.exe"), dir.join(".fox-git.exe.part"));
        std::fs::write(&target, b"old").unwrap();
        let mut expected = update(b"new version");
        expected.sha256 = "0".repeat(64);
        assert!(save(&b"new version"[..], &expected, &part, &|_| {}).is_err());
        assert!(save(&b"new version and more"[..], &update(b"new version"), &part, &|_| {}).is_err());
        assert!(!part.exists());
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_good_download_replaces_the_file_under_its_name() {
        let dir = std::env::temp_dir().join("fox-git-update-test-good");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (target, part) = (dir.join("fox-git.exe"), dir.join(".fox-git.exe.part"));
        std::fs::write(&target, b"old").unwrap();
        let seen = std::cell::Cell::new(0);
        save(&b"new version"[..], &update(b"new version"), &part, &|done| seen.set(done)).unwrap();
        assert_eq!(seen.get(), 11);
        replace_running(&part, &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new version");
        assert_eq!(std::fs::read(old_copy(&target)).unwrap(), b"old");
        assert!(!part.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
