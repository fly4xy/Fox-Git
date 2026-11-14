use crate::git::Res;
use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::thread;
use tauri::{State, ipc::Channel};

struct Session {
    input: mpsc::Sender<Vec<u8>>,
    master: Box<dyn MasterPty + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

/// Live shells by id. A shell keeps running while its repository tab is in the background.
#[derive(Clone, Default)]
pub struct Terminals(Arc<Mutex<HashMap<u32, Session>>>);

#[cfg(windows)]
fn git_bash() -> Option<std::path::PathBuf> {
    let on_path = std::env::split_paths(&std::env::var_os("PATH")?)
        .find(|dir| dir.join("git.exe").is_file())?;
    on_path
        .ancestors()
        .flat_map(|dir| [dir.join("bin/bash.exe"), dir.join("usr/bin/bash.exe")])
        .find(|bash| bash.is_file())
}

#[cfg(windows)]
fn shell() -> CommandBuilder {
    let installed = std::env::var_os("ProgramFiles")
        .map(|dir| std::path::Path::new(&dir).join("Git/bin/bash.exe"))
        .filter(|bash| bash.is_file());
    match git_bash().or(installed) {
        Some(bash) => {
            let mut cmd = CommandBuilder::new(bash);
            cmd.args(["--login", "-i"]);
            cmd.env("CHERE_INVOKING", "1"); // keeps the login profile from leaving `cwd`
            cmd
        }
        None => CommandBuilder::new("powershell.exe"),
    }
}

#[cfg(not(windows))]
fn shell() -> CommandBuilder {
    CommandBuilder::new_default_prog()
}

impl Terminals {
    fn sessions(&self) -> MutexGuard<'_, HashMap<u32, Session>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Starts a shell. `sink` gets its output, then an empty chunk when it exits.
    fn start<S>(&self, id: u32, cwd: &str, cols: u16, rows: u16, sink: S) -> Res<()>
    where
        S: Fn(Vec<u8>) + Clone + Send + 'static,
    {
        let size = PtySize { rows, cols, pixel_width: 0, pixel_height: 0 };
        let pair = native_pty_system().openpty(size).map_err(|e| e.to_string())?;
        let mut cmd = shell();
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        let mut child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let mut writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        let (input, queue) = mpsc::channel::<Vec<u8>>();
        let killer = child.clone_killer();
        self.sessions().insert(id, Session { input, master: pair.master, killer });

        // A full pipe blocks the writer, which must not stall the app.
        thread::spawn(move || {
            for data in queue {
                if writer.write_all(&data).and_then(|_| writer.flush()).is_err() {
                    break;
                }
            }
        });
        let output = sink.clone();
        thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            while let Ok(n) = reader.read(&mut buffer) {
                if n == 0 {
                    break;
                }
                output(buffer[..n].to_vec());
            }
        });
        // The reader alone is not enough: ConPTY keeps its pipe open after the shell exits.
        let terminals = self.clone();
        thread::spawn(move || {
            let _ = child.wait();
            terminals.sessions().remove(&id);
            sink(Vec::new());
        });
        Ok(())
    }

    fn write(&self, id: u32, data: &[u8]) {
        if let Some(session) = self.sessions().get(&id) {
            let _ = session.input.send(data.to_vec());
        }
    }

    fn close(&self, id: u32) {
        if let Some(mut session) = self.sessions().remove(&id) {
            let _ = session.killer.kill();
        }
    }
}

#[tauri::command(async)]
pub fn terminal_open(
    id: u32,
    cwd: String,
    cols: u16,
    rows: u16,
    output: Channel<Vec<u8>>,
    terminals: State<'_, Terminals>,
) -> Res<()> {
    terminals.start(id, &cwd, cols, rows, move |data| {
        let _ = output.send(data);
    })
}

/// Stays synchronous so that keystrokes reach the shell in the order they were typed.
#[tauri::command]
pub fn terminal_write(id: u32, data: String, terminals: State<'_, Terminals>) {
    terminals.write(id, data.as_bytes());
}

#[tauri::command]
pub fn terminal_resize(id: u32, cols: u16, rows: u16, terminals: State<'_, Terminals>) -> Res<()> {
    let size = PtySize { rows, cols, pixel_width: 0, pixel_height: 0 };
    match terminals.sessions().get(&id) {
        Some(session) => session.master.resize(size).map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

#[tauri::command]
pub fn terminal_close(id: u32, terminals: State<'_, Terminals>) {
    terminals.close(id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Plays the terminal: answers the cursor query ConPTY sends, collects the output.
    fn wait_for(terminals: &Terminals, seen: &mpsc::Receiver<Vec<u8>>, text: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut all = String::new();
        while Instant::now() < deadline {
            let Ok(chunk) = seen.recv_timeout(Duration::from_millis(200)) else { continue };
            if chunk.is_empty() {
                all.push_str("<exit>");
            }
            let chunk = String::from_utf8_lossy(&chunk);
            if chunk.contains("\x1b[6n") {
                terminals.write(1, b"\x1b[1;1R");
            }
            all.push_str(&chunk);
            if all.contains(text) {
                return all;
            }
        }
        panic!("no {text:?} in {all:?}");
    }

    #[test]
    fn shell_starts_in_the_repository_and_reports_exit() {
        let dir = std::env::temp_dir().join("fox-git-terminal-test");
        std::fs::create_dir_all(&dir).unwrap();
        let (sender, seen) = mpsc::channel();
        let sink = move |data| {
            let _ = sender.send(data);
        };
        let terminals = Terminals::default();
        terminals.start(1, &dir.to_string_lossy(), 80, 24, sink).unwrap();
        terminals.write(1, b"echo fox-$((40+2)) $(basename \"$PWD\")\r");
        wait_for(&terminals, &seen, "fox-42 fox-git-terminal-test");
        terminals.write(1, b"exit\r");
        wait_for(&terminals, &seen, "<exit>");
        assert!(terminals.sessions().is_empty());
    }
}
