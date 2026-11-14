use crate::git::{Res, hidden};
use serde::Serialize;
use std::path::PathBuf;
use std::process::Stdio;

fn home() -> Res<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| "Home folder not found".to_string())
}

fn output(program: &str, args: &[&str]) -> Option<String> {
    let out = hidden(program).args(args).stdin(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(windows)]
fn accent() -> Option<String> {
    let raw = output("reg", &["query", r"HKCU\Software\Microsoft\Windows\DWM", "/v", "AccentColor"])?;
    let hex = raw.split_whitespace().last()?.strip_prefix("0x")?;
    parse_abgr(u32::from_str_radix(hex, 16).ok()?)
}

#[cfg(any(windows, test))]
fn parse_abgr(value: u32) -> Option<String> {
    let (r, g, b) = (value & 0xff, (value >> 8) & 0xff, (value >> 16) & 0xff);
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

#[cfg(target_os = "macos")]
fn accent() -> Option<String> {
    let value = output("defaults", &["read", "-g", "AppleAccentColor"]);
    let color = match value.as_deref() {
        Some("-1") => "#8c8c8c",
        Some("0") => "#e0383e",
        Some("1") => "#f7821b",
        Some("2") => "#fcb827",
        Some("3") => "#62ba46",
        Some("5") => "#953d96",
        Some("6") => "#f74f9e",
        _ => "#007aff",
    };
    Some(color.to_string())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn accent() -> Option<String> {
    kde_accent().or_else(gnome_accent)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn kde_accent() -> Option<String> {
    let config = std::fs::read_to_string(home().ok()?.join(".config/kdeglobals")).ok()?;
    let rgb = config.lines().find_map(|l| l.strip_prefix("AccentColor="))?;
    let c: Vec<u8> = rgb.split(',').filter_map(|n| n.trim().parse().ok()).collect();
    let [r, g, b] = c[..] else { return None };
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn gnome_accent() -> Option<String> {
    let name = output("gsettings", &["get", "org.gnome.desktop.interface", "accent-color"])?;
    let color = match name.trim_matches('\'') {
        "blue" => "#3584e4",
        "teal" => "#2190a4",
        "green" => "#3a944a",
        "yellow" => "#c88800",
        "orange" => "#ed5b00",
        "red" => "#e62d42",
        "pink" => "#d56199",
        "purple" => "#9141ac",
        "slate" => "#6f8396",
        _ => return None,
    };
    Some(color.to_string())
}

#[tauri::command(async)]
pub fn system_accent() -> Option<String> {
    accent()
}

#[derive(Serialize)]
pub struct SshKey {
    name: String,
    public_key: String,
}

#[tauri::command(async)]
pub fn ssh_keys() -> Res<Vec<SshKey>> {
    let Ok(dir) = std::fs::read_dir(home()?.join(".ssh")) else { return Ok(vec![]) };
    let mut keys: Vec<SshKey> = dir
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "pub"))
        .filter_map(|e| {
            Some(SshKey {
                name: e.path().file_stem()?.to_string_lossy().into_owned(),
                public_key: std::fs::read_to_string(e.path()).ok()?.trim().to_string(),
            })
        })
        .collect();
    keys.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(keys)
}

#[tauri::command(async)]
pub fn ssh_generate(name: String, comment: String) -> Res<()> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c)) {
        return Err("Key name may use Latin letters, digits, dot, dash and underscore".into());
    }
    let dir = home()?.join(".ssh");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(&name);
    if path.exists() {
        return Err(format!("Key {name} already exists"));
    }
    let out = hidden("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-C", &comment, "-f"])
        .arg(&path)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Cannot run ssh-keygen: {e}"))?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

/// GitHub answers `ssh -T` with a greeting and exit code 1 even on success.
#[tauri::command(async)]
pub fn ssh_test() -> Res<String> {
    let out = hidden("ssh")
        .args(["-T", "-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=accept-new", "git@github.com"])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Cannot run ssh: {e}"))?;
    let message = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if message.contains("successfully authenticated") { Ok(message) } else { Err(message) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_accent_is_stored_as_abgr() {
        assert_eq!(parse_abgr(0xffd77800).as_deref(), Some("#0078d7"));
    }
}
