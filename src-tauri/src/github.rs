use crate::git::Res;
use base64::Engine;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

const SCOPES: &str = "repo read:user write:public_key";
const USER_AGENT: &str = "Fox Git";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

/// Set at build time from the environment or the project's `.env`, see build.rs.
const BUILT_IN_ID: Option<&str> = option_env!("FOXGIT_GITHUB_CLIENT_ID");
const BUILT_IN_SECRET: Option<&str> = option_env!("FOXGIT_GITHUB_CLIENT_SECRET");

static CANCEL_LOGIN: AtomicBool = AtomicBool::new(false);

fn entry(name: &str) -> Res<keyring::Entry> {
    keyring::Entry::new("fox-git", name).map_err(|e| e.to_string())
}

pub fn token() -> Option<String> {
    entry("github").ok()?.get_password().ok()
}

fn required_token() -> Res<String> {
    token().ok_or_else(|| "Not signed in to GitHub".to_string())
}

fn describe(err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, resp) => {
            let body: Value = resp.into_json().unwrap_or_default();
            let message = body["message"].as_str().unwrap_or("request rejected");
            format!("GitHub {code}: {message}")
        }
        other => format!("Cannot reach GitHub: {other}"),
    }
}

fn api_get(path: &str, token: &str) -> Result<Value, ureq::Error> {
    Ok(ureq::get(&format!("https://api.github.com{path}"))
        .set("Accept", "application/vnd.github+json")
        .set("Authorization", &format!("Bearer {token}"))
        .set("User-Agent", USER_AGENT)
        .call()?
        .into_json()?)
}

fn random_token() -> Res<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

fn pkce_challenge(verifier: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn query_param<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    query.split('&').find_map(|pair| pair.split_once('=').filter(|(key, _)| *key == name).map(|(_, value)| value))
}

/// The query string of a `GET /callback?...` request line.
fn callback_query(request: &str) -> Option<&str> {
    request.lines().next()?.strip_prefix("GET /callback?")?.split(' ').next()
}

fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    let page = format!(
        "<!doctype html><meta charset=utf-8><title>Fox Git</title>\
         <body style=\"font:16px system-ui;background:#141415;color:#ededed;display:grid;place-items:center;height:90vh\">\
         <p>{body}</p>"
    );
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
        page.len()
    );
}

/// Waits for GitHub to send the browser back to the local redirect and returns the code.
fn wait_for_code(listener: &TcpListener, state: &str, done_page: &str) -> Res<String> {
    let deadline = Instant::now() + LOGIN_TIMEOUT;
    while Instant::now() < deadline {
        if CANCEL_LOGIN.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        let Ok((mut stream, _)) = listener.accept() else {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        };
        stream.set_nonblocking(false).map_err(|e| e.to_string())?;
        stream.set_read_timeout(Some(Duration::from_secs(2))).map_err(|e| e.to_string())?;
        let mut buffer = [0u8; 4096];
        let size = stream.read(&mut buffer).unwrap_or(0);
        let request = String::from_utf8_lossy(&buffer[..size]);
        let Some(query) = callback_query(&request) else {
            respond(&mut stream, "404 Not Found", "Not found");
            continue;
        };
        if query_param(query, "state") != Some(state) {
            respond(&mut stream, "400 Bad Request", "Invalid state");
            continue;
        }
        if let Some(code) = query_param(query, "code") {
            respond(&mut stream, "200 OK", done_page);
            return Ok(code.to_string());
        }
        respond(&mut stream, "200 OK", "Access was not granted");
        return Err("GitHub access was not granted".into());
    }
    Err("Timed out waiting for the browser".into())
}

fn credentials(client_id: String, client_secret: String) -> Res<(String, String)> {
    if let (Some(id), Some(secret)) = (BUILT_IN_ID, BUILT_IN_SECRET) {
        return Ok((id.into(), secret.into()));
    }
    let stored = entry("github-client-secret")?;
    if !client_secret.is_empty() {
        stored.set_password(&client_secret).map_err(|e| e.to_string())?;
    }
    let secret = stored.get_password().map_err(|_| "GitHub client secret is missing".to_string())?;
    Ok((client_id, secret))
}

#[tauri::command]
pub fn github_built_in() -> bool {
    BUILT_IN_ID.is_some_and(|id| !id.is_empty()) && BUILT_IN_SECRET.is_some_and(|s| !s.is_empty())
}

/// OAuth web flow with a loopback redirect: the user approves access in the browser.
#[tauri::command(async)]
pub fn github_login(app: AppHandle, client_id: String, client_secret: String, done_page: String) -> Res<()> {
    let (client_id, client_secret) = credentials(client_id, client_secret)?;
    CANCEL_LOGIN.store(false, Ordering::Relaxed);

    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}/callback");
    let state = random_token()?;
    let verifier = random_token()?;

    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        encode(&client_id),
        encode(&redirect),
        encode(SCOPES),
        state,
        pkce_challenge(&verifier),
    );
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())?;

    let code = wait_for_code(&listener, &state, &done_page)?;
    let body: Value = ureq::post("https://github.com/login/oauth/access_token")
        .set("Accept", "application/json")
        .set("User-Agent", USER_AGENT)
        .send_form(&[
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("code", &code),
            ("redirect_uri", &redirect),
            ("code_verifier", &verifier),
        ])
        .map_err(describe)?
        .into_json()
        .map_err(|e| e.to_string())?;
    let Some(token) = body["access_token"].as_str() else {
        return Err(body["error_description"].as_str().unwrap_or("Sign-in failed").to_string());
    };
    entry("github")?.set_password(token).map_err(|e| e.to_string())?;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_focus();
    }
    Ok(())
}

#[tauri::command]
pub fn github_login_cancel() {
    CANCEL_LOGIN.store(true, Ordering::Relaxed);
}

#[derive(Serialize)]
pub struct GithubUser {
    login: String,
    name: Option<String>,
    avatar_url: String,
}

#[tauri::command(async)]
pub fn github_user() -> Res<Option<GithubUser>> {
    let Some(token) = token() else { return Ok(None) };
    let user = api_get("/user", &token).map_err(describe)?;
    Ok(Some(GithubUser {
        login: user["login"].as_str().unwrap_or_default().into(),
        name: user["name"].as_str().map(str::to_string),
        avatar_url: user["avatar_url"].as_str().unwrap_or_default().into(),
    }))
}

#[tauri::command(async)]
pub fn github_logout() -> Res<()> {
    match entry("github")?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[derive(Serialize)]
pub struct GithubRepo {
    full_name: String,
    clone_url: String,
    ssh_url: String,
    private: bool,
}

#[tauri::command(async)]
pub fn github_repos() -> Res<Vec<GithubRepo>> {
    let token = required_token()?;
    let repos = api_get("/user/repos?per_page=100&sort=pushed", &token).map_err(describe)?;
    let list = repos.as_array().cloned().unwrap_or_default();
    Ok(list
        .iter()
        .map(|r| GithubRepo {
            full_name: r["full_name"].as_str().unwrap_or_default().into(),
            clone_url: r["clone_url"].as_str().unwrap_or_default().into(),
            ssh_url: r["ssh_url"].as_str().unwrap_or_default().into(),
            private: r["private"].as_bool().unwrap_or(false),
        })
        .collect())
}

#[tauri::command(async)]
pub fn github_add_ssh_key(title: String, key: String) -> Res<()> {
    let token = required_token()?;
    ureq::post("https://api.github.com/user/keys")
        .set("Accept", "application/vnd.github+json")
        .set("Authorization", &format!("Bearer {token}"))
        .set("User-Agent", USER_AGENT)
        .send_json(json!({ "title": title, "key": key }))
        .map_err(describe)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_the_rfc_example() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(pkce_challenge(verifier), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn callback_request_is_parsed() {
        let request = "GET /callback?code=abc&state=xyz HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let query = callback_query(request).unwrap();
        assert_eq!(query_param(query, "code"), Some("abc"));
        assert_eq!(query_param(query, "state"), Some("xyz"));
        assert_eq!(callback_query("GET /favicon.ico HTTP/1.1\r\n"), None);
    }

    #[test]
    fn redirect_is_percent_encoded() {
        assert_eq!(encode("http://127.0.0.1:80/a b"), "http%3A%2F%2F127.0.0.1%3A80%2Fa%20b");
    }
}
