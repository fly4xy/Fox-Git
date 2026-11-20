use crate::github;
use serde::Serialize;
use std::io::{Read, Write};
use std::process::{Command, Output, Stdio};
use tauri::ipc::Channel;

pub type Res<T> = Result<T, String>;

const FIELD: char = '\x1f';
const RECORD: char = '\x1e';
const MAX_DIFF_BYTES: usize = 4 * 1024 * 1024;

/// Subcommands the UI may run through `git_action`.
const ALLOWED: &[&str] = &[
    "add", "apply", "branch", "checkout", "cherry-pick", "clean", "commit", "fetch", "lfs",
    "merge", "pull", "push", "rebase", "remote", "reset", "restore", "revert", "rm", "stash",
    "switch",
    "tag",
];
const NETWORK: &[&str] = &["fetch", "pull", "push", "lfs"];
/// Subcommands that report progress when asked with `--progress`.
const PROGRESS: &[&str] = &["checkout", "fetch", "pull", "push"];

#[cfg_attr(not(windows), allow(unused_mut))]
pub fn hidden(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

fn git(repo: &str) -> Command {
    let mut cmd = hidden("git");
    cmd.current_dir(repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_EDITOR", "true")
        .env("LC_ALL", "C")
        .env("GIT_LITERAL_PATHSPECS", "1")
        .args(["-c", "core.quotepath=false", "-c", "color.ui=false"]);
    cmd
}

/// Lets git reach remotes without a terminal: GitHub token over HTTPS, no prompts over SSH.
fn with_network(cmd: &mut Command) {
    if std::env::var_os("GIT_SSH_COMMAND").is_none() {
        cmd.env(
            "GIT_SSH_COMMAND",
            "ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new",
        );
    }
    if let Some(token) = github::token() {
        cmd.env("FOXGIT_TOKEN", token).args([
            "-c",
            "credential.https://github.com.helper=",
            "-c",
            "credential.https://github.com.helper=!f() { test \"$1\" = get && echo username=x-access-token && echo password=$FOXGIT_TOKEN; }; f",
        ]);
    }
}

/// Splits stderr into lines as it arrives. Git redraws progress with `\r`, and those
/// redrawn lines stay out of the error text.
#[derive(Default)]
struct Lines {
    line: Vec<u8>,
    kept: Vec<u8>,
}

impl Lines {
    fn feed(&mut self, chunk: &[u8], on_line: &mut impl FnMut(String)) {
        for &byte in chunk {
            if byte == b'\r' || byte == b'\n' {
                self.end(byte == b'\n', on_line);
            } else {
                self.line.push(byte);
            }
        }
    }

    fn end(&mut self, kept: bool, on_line: &mut impl FnMut(String)) {
        let line = text(&self.line).trim().to_string();
        if !line.is_empty() {
            if kept && !is_progress(&line) {
                self.kept.extend_from_slice(line.as_bytes());
                self.kept.push(b'\n');
            }
            on_line(line);
        }
        self.line.clear();
    }
}

fn is_progress(line: &str) -> bool {
    line.contains("% (") || line.ends_with(", done.")
}

/// Runs a command and passes each stderr line to `on_line` while it runs.
fn exec_with(mut cmd: Command, stdin: Option<String>, mut on_line: impl FnMut(String)) -> Res<Output> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() });
    let mut child = cmd.spawn().map_err(|e| format!("Cannot run git: {e}"))?;
    if let Some(input) = stdin {
        let mut pipe = child.stdin.take().ok_or("stdin is not available")?;
        std::thread::spawn(move || {
            let _ = pipe.write_all(input.as_bytes());
        });
    }
    let mut stdout = child.stdout.take().ok_or("stdout is not available")?;
    let reading = std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out);
        out
    });
    let mut stderr = child.stderr.take().ok_or("stderr is not available")?;
    let mut lines = Lines::default();
    let mut buffer = [0u8; 4096];
    while let Ok(n @ 1..) = stderr.read(&mut buffer) {
        lines.feed(&buffer[..n], &mut on_line);
    }
    lines.end(true, &mut on_line);
    let status = child.wait().map_err(|e| e.to_string())?;
    let stdout = reading.join().unwrap_or_default();
    Ok(Output { status, stdout, stderr: lines.kept })
}

fn exec(cmd: Command, stdin: Option<String>) -> Res<Output> {
    exec_with(cmd, stdin, |_| ())
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn failure(out: &Output) -> String {
    let err = text(&out.stderr);
    let err = err.trim();
    if err.is_empty() { text(&out.stdout).trim().to_string() } else { err.to_string() }
}

fn run(repo: &str, args: &[&str]) -> Res<String> {
    let mut cmd = git(repo);
    cmd.args(args);
    let out = exec(cmd, None)?;
    if out.status.success() { Ok(text(&out.stdout)) } else { Err(failure(&out)) }
}

fn run_diff(repo: &str, args: &[&str]) -> Res<String> {
    let mut cmd = git(repo);
    cmd.args(args);
    let out = exec(cmd, None)?;
    // `diff --no-index` exits with 1 when the files differ.
    if out.status.code().is_some_and(|c| c <= 1) {
        let end = out.stdout.len().min(MAX_DIFF_BYTES);
        Ok(text(&out.stdout[..end]))
    } else {
        Err(failure(&out))
    }
}

#[tauri::command(async)]
pub fn repo_root(path: String) -> Res<String> {
    let path = std::path::Path::new(&path);
    let dir = if path.is_file() { path.parent().unwrap_or(path) } else { path };
    Ok(run(&dir.to_string_lossy(), &["rev-parse", "--show-toplevel"])?.trim().to_string())
}

/// Repositories directly inside a folder, for when a parent folder was picked.
#[tauri::command(async)]
pub fn repos_in(path: String) -> Res<Vec<String>> {
    let dir = std::fs::read_dir(&path).map_err(|e| e.to_string())?;
    let mut repos: Vec<String> = dir
        .flatten()
        .filter(|e| e.path().join(".git").exists())
        .map(|e| e.path().to_string_lossy().into_owned())
        .collect();
    repos.sort();
    Ok(repos)
}

/// Marks a folder owned by another user as safe for git.
#[tauri::command(async)]
pub fn trust_repo(path: String) -> Res<()> {
    let cwd = std::env::temp_dir();
    run(&cwd.to_string_lossy(), &["config", "--global", "--add", "safe.directory", &path]).map(|_| ())
}

/// Adds the patterns `.gitignore` lacks, then tells which of `paths` it ignores,
/// tracked files included.
#[tauri::command(async)]
pub fn ignore_paths(repo: String, patterns: Vec<String>, paths: Vec<String>) -> Res<Vec<String>> {
    let file = std::path::Path::new(&repo).join(".gitignore");
    let current = std::fs::read_to_string(&file).unwrap_or_default();
    let newline = if current.contains("\r\n") { "\r\n" } else { "\n" };
    let mut written = current.clone();
    for pattern in &patterns {
        if current.lines().any(|line| line.trim_end_matches('\r') == pattern) {
            continue;
        }
        if !written.is_empty() && !written.ends_with('\n') {
            written.push_str(newline);
        }
        written.push_str(pattern);
        written.push_str(newline);
    }
    if written != current {
        std::fs::write(&file, written).map_err(|e| e.to_string())?;
    }
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let mut cmd = git(&repo);
    // check-ignore refuses literal pathspecs, and its paths are plain paths anyway.
    cmd.env_remove("GIT_LITERAL_PATHSPECS");
    cmd.args(["check-ignore", "--no-index", "--stdin", "-z"]);
    let out = exec(cmd, Some(paths.join("\0") + "\0"))?;
    // Exit code 1 means that none of the paths is ignored.
    match out.status.code() {
        Some(0 | 1) => Ok(text(&out.stdout).split('\0').filter(|p| !p.is_empty()).map(String::from).collect()),
        _ => Err(failure(&out)),
    }
}

#[derive(Serialize)]
pub struct Branch {
    name: String,
    upstream: String,
    ahead: u32,
    behind: u32,
    hash: String,
    current: bool,
    /// Checked out in another worktree, so git refuses to switch to it or delete it.
    elsewhere: bool,
}

#[derive(Serialize)]
pub struct Stash {
    name: String,
    hash: String,
    message: String,
}

#[derive(Serialize)]
pub struct Overview {
    head: Option<String>,
    head_hash: Option<String>,
    branches: Vec<Branch>,
    remote_branches: Vec<String>,
    tags: Vec<String>,
    stashes: Vec<Stash>,
    remotes: Vec<String>,
}

fn parse_track(track: &str) -> (u32, u32) {
    let (mut ahead, mut behind) = (0, 0);
    for part in track.split(", ") {
        if let Some(n) = part.strip_prefix("ahead ") {
            ahead = n.parse().unwrap_or(0);
        } else if let Some(n) = part.strip_prefix("behind ") {
            behind = n.parse().unwrap_or(0);
        }
    }
    (ahead, behind)
}

fn parse_branches(raw: &str) -> Vec<Branch> {
    raw.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(FIELD).collect();
            let [head, name, upstream, track, hash, worktree] = f[..] else { return None };
            let (ahead, behind) = parse_track(track);
            Some(Branch {
                name: name.into(),
                upstream: upstream.into(),
                ahead,
                behind,
                hash: hash.into(),
                current: head == "*",
                elsewhere: head != "*" && !worktree.is_empty(),
            })
        })
        .collect()
}

fn lines(raw: String) -> Vec<String> {
    raw.lines().map(str::to_string).collect()
}

#[tauri::command(async)]
pub fn overview(repo: String) -> Res<Overview> {
    let head_hash = run(&repo, &["rev-parse", "--verify", "-q", "HEAD"]).ok().map(|s| s.trim().to_string());
    let head = run(&repo, &["symbolic-ref", "--short", "-q", "HEAD"]).ok().map(|s| s.trim().to_string());
    let branches = run(
        &repo,
        &[
            "for-each-ref",
            "refs/heads",
            "--format=%(HEAD)%1f%(refname:short)%1f%(upstream:short)%1f%(upstream:track,nobracket)%1f%(objectname)%1f%(worktreepath)",
        ],
    )?;
    let remote_branches = run(&repo, &["for-each-ref", "refs/remotes", "--format=%(refname:short)"])?;
    let tags = run(&repo, &["for-each-ref", "refs/tags", "--sort=-creatordate", "--format=%(refname:short)"])?;
    let stashes = run(&repo, &["stash", "list", "--format=%gd%x1f%H%x1f%s"])?
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(FIELD).collect();
            let [name, hash, message] = f[..] else { return None };
            Some(Stash { name: name.into(), hash: hash.into(), message: message.into() })
        })
        .collect();
    Ok(Overview {
        head,
        head_hash,
        branches: parse_branches(&branches),
        remote_branches: lines(remote_branches)
            .into_iter()
            .filter(|b| !b.ends_with("/HEAD") && b.contains('/'))
            .collect(),
        tags: lines(tags),
        stashes,
        remotes: lines(run(&repo, &["remote"])?),
    })
}

#[derive(Serialize)]
pub struct Commit {
    hash: String,
    parents: Vec<String>,
    author: String,
    email: String,
    time: i64,
    refs: String,
    subject: String,
}

fn parse_log(raw: &str) -> Vec<Commit> {
    raw.split(RECORD)
        .filter_map(|rec| {
            let f: Vec<&str> = rec.trim_start_matches('\n').split(FIELD).collect();
            let [hash, parents, author, email, time, refs, subject] = f[..] else { return None };
            Some(Commit {
                hash: hash.into(),
                parents: parents.split_whitespace().map(str::to_string).collect(),
                author: author.into(),
                email: email.into(),
                time: time.parse().unwrap_or(0),
                refs: refs.into(),
                subject: subject.into(),
            })
        })
        .collect()
}

#[tauri::command(async)]
pub fn log(repo: String, limit: u32, skip: u32) -> Res<Vec<Commit>> {
    if run(&repo, &["rev-parse", "--verify", "-q", "HEAD"]).is_err()
        && run(&repo, &["for-each-ref", "--count=1"])?.is_empty()
    {
        return Ok(vec![]);
    }
    let raw = run(
        &repo,
        &[
            "-c",
            "log.showSignature=false",
            "log",
            "--exclude=refs/stash",
            "--all",
            "--date-order",
            &format!("-n{limit}"),
            &format!("--skip={skip}"),
            "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%D%x1f%s%x1e",
        ],
    )?;
    Ok(parse_log(&raw))
}

#[derive(Serialize)]
pub struct FileCommit {
    #[serde(flatten)]
    commit: Commit,
    /// The file's name in this commit, which differs from today's after a rename.
    path: String,
    old_path: Option<String>,
}

/// Commits that changed one file, newest first, following it across renames.
#[tauri::command(async)]
pub fn file_log(repo: String, path: String) -> Res<Vec<FileCommit>> {
    let raw = run(
        &repo,
        &[
            "-c",
            "log.showSignature=false",
            "log",
            "--follow",
            "-M",
            "-n1000",
            "--name-status",
            "-z",
            "--format=%x1e%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%D%x1f%s%x1f",
            "--",
            &path,
        ],
    )?;
    let mut name = path;
    let mut commits = vec![];
    for record in raw.split(RECORD) {
        let fields: Vec<&str> = record.splitn(8, FIELD).collect();
        let [hash, parents, author, email, time, refs, subject, names] = fields[..] else { continue };
        // Merges list no files, so they keep the name of the newer commit.
        let (path, old_path) = match parse_name_status(names.trim_start_matches(['\n', '\0'])).into_iter().next() {
            Some(change) => (change.path, change.old_path),
            None => (name.clone(), None),
        };
        name = old_path.clone().unwrap_or_else(|| path.clone());
        commits.push(FileCommit {
            commit: Commit {
                hash: hash.into(),
                parents: parents.split_whitespace().map(str::to_string).collect(),
                author: author.into(),
                email: email.into(),
                time: time.parse().unwrap_or(0),
                refs: refs.into(),
                subject: subject.into(),
            },
            path,
            old_path,
        });
    }
    Ok(commits)
}

#[derive(Serialize, Clone)]
pub struct FileChange {
    path: String,
    old_path: Option<String>,
    code: String,
}

#[derive(Serialize)]
pub struct Status {
    staged: Vec<FileChange>,
    unstaged: Vec<FileChange>,
    conflicted: Vec<FileChange>,
    state: Option<String>,
    /// The message git prepared for the commit that finishes a merge, cherry-pick or revert.
    message: Option<String>,
}

fn parse_status(raw: &str) -> (Vec<FileChange>, Vec<FileChange>, Vec<FileChange>) {
    let (mut staged, mut unstaged, mut conflicted) = (vec![], vec![], vec![]);
    let mut items = raw.split('\0');
    while let Some(item) = items.next() {
        let Some(kind) = item.chars().next() else { continue };
        match kind {
            '1' | '2' => {
                let fields = if kind == '1' { 9 } else { 10 };
                let parts: Vec<&str> = item.splitn(fields, ' ').collect();
                if parts.len() < fields {
                    continue;
                }
                let path = parts[fields - 1].to_string();
                let old_path = if kind == '2' { items.next().map(str::to_string) } else { None };
                let mut xy = parts[1].chars();
                let (x, y) = (xy.next().unwrap_or('.'), xy.next().unwrap_or('.'));
                if x != '.' {
                    staged.push(FileChange { path: path.clone(), old_path: old_path.clone(), code: x.to_string() });
                }
                if y != '.' {
                    unstaged.push(FileChange { path, old_path, code: y.to_string() });
                }
            }
            'u' => {
                if let Some(path) = item.splitn(11, ' ').nth(10) {
                    conflicted.push(FileChange { path: path.into(), old_path: None, code: "U".into() });
                }
            }
            '?' => unstaged.push(FileChange { path: item[2..].into(), old_path: None, code: "?".into() }),
            _ => {}
        }
    }
    (staged, unstaged, conflicted)
}

fn repo_state(repo: &str) -> Option<String> {
    let markers = [
        ("rebase-merge", "rebase"),
        ("rebase-apply", "rebase"),
        ("MERGE_HEAD", "merge"),
        ("CHERRY_PICK_HEAD", "cherry-pick"),
        ("REVERT_HEAD", "revert"),
    ];
    markers.iter().find_map(|(file, state)| {
        let path = run(repo, &["rev-parse", "--git-path", file]).ok()?;
        std::path::Path::new(repo).join(path.trim()).exists().then(|| state.to_string())
    })
}

#[tauri::command(async)]
pub fn status(repo: String) -> Res<Status> {
    let raw = run(&repo, &["status", "--porcelain=v2", "-z", "--untracked-files=all"])?;
    let (staged, unstaged, conflicted) = parse_status(&raw);
    let state = repo_state(&repo);
    let message = state.as_deref().filter(|s| *s != "rebase").and_then(|_| prepared_message(&repo));
    Ok(Status { staged, unstaged, conflicted, state, message })
}

/// `MERGE_MSG` without its comment lines. Git writes it in English, as commands run with `LC_ALL=C`.
fn prepared_message(repo: &str) -> Option<String> {
    let path = run(repo, &["rev-parse", "--git-path", "MERGE_MSG"]).ok()?;
    let raw = std::fs::read_to_string(std::path::Path::new(repo).join(path.trim())).ok()?;
    let kept: Vec<&str> = raw.lines().filter(|line| !line.starts_with('#')).collect();
    let message = kept.join("\n").trim().to_string();
    (!message.is_empty()).then_some(message)
}

#[derive(Serialize)]
pub struct ConflictSide {
    name: String,
    /// The last commit on this side that changed the file.
    time: Option<i64>,
    commit: Option<String>,
    /// Empty when this side deleted the file.
    size: Option<u64>,
}

#[derive(Serialize)]
pub struct Conflict {
    ours: ConflictSide,
    theirs: ConflictSide,
    /// From the current version to the incoming one; empty when one side deleted the file.
    diff: String,
}

/// Both versions of a conflicted file: when each side last changed it, and how they differ.
#[tauri::command(async)]
pub fn conflict_diff(repo: String, path: String) -> Res<Conflict> {
    let incoming = ["MERGE_HEAD", "REBASE_HEAD", "CHERRY_PICK_HEAD", "REVERT_HEAD"]
        .into_iter()
        .find(|name| run(&repo, &["rev-parse", "-q", "--verify", name]).is_ok())
        .unwrap_or("MERGE_HEAD");
    let side = |name: &str, stage: u8| {
        let last = run(&repo, &["log", "-1", "--format=%ct%x1f%h %s", name, "--", &path]).unwrap_or_default();
        let (time, commit) = match last.trim().split_once('\x1f') {
            Some((time, commit)) => (time.parse().ok(), Some(commit.to_string())),
            None => (None, None),
        };
        let blob = format!(":{stage}:{path}");
        let size = run(&repo, &["cat-file", "-s", &blob]).ok().and_then(|s| s.trim().parse().ok());
        ConflictSide { name: name.to_string(), time, commit, size }
    };
    let (ours, theirs) = (side("HEAD", 2), side(incoming, 3));
    let diff = if ours.size.is_some() && theirs.size.is_some() {
        run_diff(&repo, &["diff", &format!(":2:{path}"), &format!(":3:{path}")])?
    } else {
        String::new()
    };
    Ok(Conflict { ours, theirs, diff })
}

#[tauri::command(async)]
pub fn file_diff(repo: String, path: String, staged: bool, untracked: bool) -> Res<String> {
    if untracked {
        run_diff(&repo, &["diff", "--no-index", "--", "/dev/null", &path])
    } else if staged {
        run_diff(&repo, &["diff", "--cached", "--", &path])
    } else {
        run_diff(&repo, &["diff", "--", &path])
    }
}

#[derive(Serialize)]
pub struct CommitDetail {
    hash: String,
    parents: Vec<String>,
    author: String,
    email: String,
    time: i64,
    message: String,
    files: Vec<FileChange>,
}

fn parse_name_status(raw: &str) -> Vec<FileChange> {
    let mut files = vec![];
    let mut items = raw.split('\0').filter(|s| !s.is_empty());
    while let Some(status) = items.next() {
        let code = status[..1].to_string();
        if code == "R" || code == "C" {
            let (Some(old), Some(new)) = (items.next(), items.next()) else { break };
            files.push(FileChange { path: new.into(), old_path: Some(old.into()), code });
        } else if let Some(path) = items.next() {
            files.push(FileChange { path: path.into(), old_path: None, code });
        }
    }
    files
}

#[tauri::command(async)]
pub fn commit_show(repo: String, hash: String) -> Res<CommitDetail> {
    let raw = run(&repo, &["show", "--no-patch", "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%B", &hash])?;
    let f: Vec<&str> = raw.splitn(6, FIELD).collect();
    let [hash, parents, author, email, time, message] = f[..] else {
        return Err("Cannot read the commit".into());
    };
    let parents: Vec<String> = parents.split_whitespace().map(str::to_string).collect();
    let names = match parents.first() {
        Some(parent) => run(&repo, &["diff", "--name-status", "-M", "-z", parent, hash])?,
        None => run(&repo, &["diff-tree", "--no-commit-id", "-r", "--root", "--name-status", "-z", hash])?,
    };
    Ok(CommitDetail {
        hash: hash.into(),
        author: author.into(),
        email: email.into(),
        time: time.parse().unwrap_or(0),
        message: message.trim().into(),
        files: parse_name_status(&names),
        parents,
    })
}

#[tauri::command(async)]
pub fn commit_file_diff(
    repo: String,
    hash: String,
    parent: Option<String>,
    path: String,
    old_path: Option<String>,
) -> Res<String> {
    let mut paths = vec![path.as_str()];
    paths.extend(old_path.as_deref());
    let mut args = match &parent {
        Some(parent) => vec!["diff", "-M", parent.as_str(), hash.as_str(), "--"],
        None => vec!["show", "--format=", hash.as_str(), "--"],
    };
    args.extend(paths);
    run_diff(&repo, &args)
}

#[derive(Serialize)]
pub struct LfsFile {
    path: String,
    size: String,
    local: bool,
}

#[derive(Serialize)]
pub struct LfsInfo {
    installed: bool,
    patterns: Vec<String>,
    files: Vec<LfsFile>,
}

fn parse_lfs_files(raw: &str) -> Vec<LfsFile> {
    raw.lines()
        .filter_map(|line| {
            let (_, rest) = line.split_once(' ')?;
            let (mark, rest) = rest.split_once(' ')?;
            let (path, size) = rest.rsplit_once(" (")?;
            Some(LfsFile { path: path.into(), size: size.trim_end_matches(')').into(), local: mark == "*" })
        })
        .collect()
}

fn parse_lfs_patterns(raw: &str) -> Vec<String> {
    raw.lines()
        .filter(|l| l.starts_with("    "))
        .filter_map(|l| l.trim().rsplit_once(" (").map(|(pattern, _)| pattern.to_string()))
        .collect()
}

#[tauri::command(async)]
pub fn lfs_info(repo: String) -> Res<LfsInfo> {
    if run(&repo, &["lfs", "version"]).is_err() {
        return Ok(LfsInfo { installed: false, patterns: vec![], files: vec![] });
    }
    Ok(LfsInfo {
        installed: true,
        patterns: parse_lfs_patterns(&run(&repo, &["lfs", "track"])?),
        files: parse_lfs_files(&run(&repo, &["lfs", "ls-files", "--size"]).unwrap_or_default()),
    })
}

#[tauri::command(async)]
pub fn git_action(
    repo: String,
    args: Vec<String>,
    stdin: Option<String>,
    progress: Channel<String>,
) -> Res<String> {
    let sub = args.first().map(String::as_str).unwrap_or_default();
    if !ALLOWED.contains(&sub) {
        return Err(format!("Command is not allowed: {sub}"));
    }
    let mut cmd = git(&repo);
    if NETWORK.contains(&sub) {
        with_network(&mut cmd);
    }
    cmd.arg(sub);
    if PROGRESS.contains(&sub) {
        cmd.arg("--progress");
    }
    cmd.args(&args[1..]);
    let out = exec_with(cmd, stdin, |line| {
        let _ = progress.send(line);
    })?;
    if out.status.success() { Ok(text(&out.stdout)) } else { Err(failure(&out)) }
}

#[tauri::command(async)]
pub fn clone_repo(url: String, dest: String, progress: Channel<String>) -> Res<String> {
    let parent = std::path::Path::new(&dest).parent().ok_or("Invalid folder")?;
    let mut cmd = git(&parent.to_string_lossy());
    with_network(&mut cmd);
    cmd.args(["clone", "--progress", "--", &url, &dest]);
    let out = exec_with(cmd, None, |line| {
        let _ = progress.send(line);
    })?;
    if out.status.success() { Ok(dest) } else { Err(failure(&out)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_history_follows_renames() {
        let dir = std::env::temp_dir().join("fox-git-file-log-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = dir.to_string_lossy().to_string();
        let git = |args: &[&str]| {
            let mut all = vec!["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"];
            all.extend_from_slice(args);
            run(&repo, &all)
        };
        git(&["init", "-q", "-b", "main"]).unwrap();
        std::fs::write(dir.join("old.txt"), "one
two
three
four
").unwrap();
        std::fs::write(dir.join("other.txt"), "x
").unwrap();
        git(&["add", "."]).unwrap();
        git(&["commit", "-q", "-m", "add"]).unwrap();
        git(&["mv", "old.txt", "new.txt"]).unwrap();
        git(&["commit", "-q", "-m", "rename"]).unwrap();
        std::fs::write(dir.join("other.txt"), "y
").unwrap();
        git(&["commit", "-q", "-am", "unrelated"]).unwrap();
        std::fs::write(dir.join("new.txt"), "one
two
three
five
").unwrap();
        git(&["commit", "-q", "-am", "edit"]).unwrap();

        let log = file_log(repo, "new.txt".into()).unwrap();
        let seen: Vec<(&str, &str)> = log.iter().map(|c| (c.commit.subject.as_str(), c.path.as_str())).collect();
        assert_eq!(seen, [("edit", "new.txt"), ("rename", "new.txt"), ("add", "old.txt")]);
        assert_eq!(log[1].old_path.as_deref(), Some("old.txt"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn conflicts_show_both_sides_and_the_prepared_message() {
        let dir = std::env::temp_dir().join("fox-git-conflict-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = dir.to_string_lossy().to_string();
        let git = |args: &[&str]| {
            let mut all = vec!["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"];
            all.extend_from_slice(args);
            run(&repo, &all)
        };
        git(&["init", "-q", "-b", "main"]).unwrap();
        std::fs::write(dir.join("a.txt"), "base\n").unwrap();
        git(&["add", "a.txt"]).unwrap();
        git(&["commit", "-q", "-m", "base"]).unwrap();
        git(&["checkout", "-q", "-b", "other"]).unwrap();
        std::fs::write(dir.join("a.txt"), "theirs\n").unwrap();
        git(&["commit", "-q", "-am", "theirs"]).unwrap();
        git(&["checkout", "-q", "main"]).unwrap();
        std::fs::write(dir.join("a.txt"), "ours\n").unwrap();
        git(&["commit", "-q", "-am", "ours"]).unwrap();
        assert!(git(&["merge", "other"]).is_err());

        let state = status(repo.clone()).unwrap();
        assert_eq!(state.conflicted.len(), 1);
        assert_eq!(state.message.as_deref(), Some("Merge branch 'other'"));

        let conflict = conflict_diff(repo, "a.txt".into()).unwrap();
        assert_eq!(conflict.theirs.name, "MERGE_HEAD");
        assert_eq!((conflict.ours.size, conflict.theirs.size), (Some(5), Some(7)));
        assert!(conflict.ours.commit.unwrap().ends_with("ours"));
        assert!(conflict.diff.contains("-ours") && conflict.diff.contains("+theirs"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ignored_paths_are_reported_once_written() {
        let dir = std::env::temp_dir().join("fox-git-ignore-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = dir.to_string_lossy().to_string();
        run(&repo, &["init", "-q"]).unwrap();
        std::fs::write(dir.join(".gitignore"), "target").unwrap();
        let paths = ["a.txt", "logs/b.log", "c.txt"].map(String::from).to_vec();
        let found = ignore_paths(repo.clone(), vec!["/a.txt".into(), "*.log".into()], paths.clone()).unwrap();
        assert_eq!(found, ["a.txt", "logs/b.log"]);
        ignore_paths(repo, vec!["*.log".into()], paths).unwrap();
        let written = std::fs::read_to_string(dir.join(".gitignore")).unwrap();
        assert_eq!(written, "target\n/a.txt\n*.log\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn progress_lines_stream_but_stay_out_of_errors() {
        let mut seen = Vec::new();
        let mut lines = Lines::default();
        let mut on_line = |line: String| seen.push(line);
        lines.feed(b"Writing objects:  50% (1/2)\rWriting obj", &mut on_line);
        lines.feed(b"ects: 100% (2/2), done.\n! [rejected] main -> main\nerror: failed", &mut on_line);
        lines.end(true, &mut on_line);
        assert_eq!(seen.len(), 4);
        assert_eq!(seen[1], "Writing objects: 100% (2/2), done.");
        assert_eq!(text(&lines.kept), "! [rejected] main -> main\nerror: failed\n");
    }

    #[test]
    fn status_parses_every_entry_kind() {
        let raw = "1 M. N... 100644 100644 100644 aaa bbb staged file.txt\0\
                   1 .M N... 100644 100644 100644 aaa aaa work.txt\0\
                   2 R. N... 100644 100644 100644 aaa aaa R100 new.txt\0old.txt\0\
                   u UU N... 100644 100644 100644 100644 a b c both.txt\0\
                   ? fresh file.txt\0";
        let (staged, unstaged, conflicted) = parse_status(raw);
        assert_eq!(staged.len(), 2);
        assert_eq!(staged[0].path, "staged file.txt");
        assert_eq!(staged[1].old_path.as_deref(), Some("old.txt"));
        assert_eq!(unstaged[0].path, "work.txt");
        assert_eq!(unstaged[1].code, "?");
        assert_eq!(unstaged[1].path, "fresh file.txt");
        assert_eq!(conflicted[0].path, "both.txt");
    }

    #[test]
    fn name_status_handles_renames() {
        let files = parse_name_status("M\0a.txt\0R090\0old.txt\0new.txt\0");
        assert_eq!(files.len(), 2);
        assert_eq!(files[1].path, "new.txt");
        assert_eq!(files[1].old_path.as_deref(), Some("old.txt"));
    }

    #[test]
    fn track_reads_ahead_and_behind() {
        assert_eq!(parse_track("ahead 2, behind 3"), (2, 3));
        assert_eq!(parse_track("gone"), (0, 0));
    }

    #[test]
    fn lfs_output_is_parsed() {
        let files = parse_lfs_files("abc123 * art/big file.psd (12 MB)\ndef456 - a.bin (1 KB)\n");
        assert_eq!(files[0].path, "art/big file.psd");
        assert_eq!(files[0].size, "12 MB");
        assert!(files[0].local);
        assert!(!files[1].local);
        let patterns = parse_lfs_patterns("Listing tracked patterns\n    *.psd (.gitattributes)\nListing excluded patterns\n");
        assert_eq!(patterns, vec!["*.psd"]);
    }
}
