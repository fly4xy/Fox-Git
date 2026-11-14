import { Channel, invoke } from '@tauri-apps/api/core';
import { record } from './log.svelte';

export interface Branch {
	name: string;
	upstream: string;
	ahead: number;
	behind: number;
	hash: string;
	current: boolean;
	elsewhere: boolean;
}
export interface Stash {
	name: string;
	hash: string;
	message: string;
}
export interface Overview {
	head: string | null;
	head_hash: string | null;
	branches: Branch[];
	remote_branches: string[];
	tags: string[];
	stashes: Stash[];
	remotes: string[];
}
export interface Commit {
	hash: string;
	parents: string[];
	author: string;
	email: string;
	time: number;
	refs: string;
	subject: string;
}
export interface FileChange {
	path: string;
	old_path: string | null;
	code: string;
}
export interface Status {
	staged: FileChange[];
	unstaged: FileChange[];
	conflicted: FileChange[];
	state: string | null;
	/** The message git prepared for the commit that finishes a merge. */
	message: string | null;
}
export interface ConflictSide {
	name: string;
	time: number | null;
	commit: string | null;
	/** Empty when this side deleted the file. */
	size: number | null;
}
export interface Conflict {
	ours: ConflictSide;
	theirs: ConflictSide;
	diff: string;
}
export interface CommitDetail {
	hash: string;
	parents: string[];
	author: string;
	email: string;
	time: number;
	message: string;
	files: FileChange[];
}
export interface LfsInfo {
	installed: boolean;
	patterns: string[];
	files: { path: string; size: string; local: boolean }[];
}
export interface GithubUser {
	login: string;
	name: string | null;
	avatar_url: string;
}
export interface GithubRepo {
	full_name: string;
	clone_url: string;
	ssh_url: string;
	private: boolean;
}
export interface SshKey {
	name: string;
	public_key: string;
}

/** Carries the lines git prints while it works. */
/** Passes git's lines on, and keeps its warnings in the log. */
function lineChannel(onLine?: (line: string) => void) {
	const channel = new Channel<string>();
	channel.onmessage = (line) => {
		if (/^warning:/i.test(line)) record('warning', line);
		onLine?.(line);
	};
	return channel;
}

export const api = {
	launchPath: () => invoke<string | null>('launch_path'),
	repoRoot: (path: string) => invoke<string>('repo_root', { path }),
	overview: (repo: string) => invoke<Overview>('overview', { repo }),
	log: (repo: string, limit: number, skip: number) =>
		invoke<Commit[]>('log', { repo, limit, skip }),
	status: (repo: string) => invoke<Status>('status', { repo }),
	fileDiff: (repo: string, path: string, staged: boolean, untracked: boolean) =>
		invoke<string>('file_diff', { repo, path, staged, untracked }),
	conflictDiff: (repo: string, path: string) => invoke<Conflict>('conflict_diff', { repo, path }),
	commitShow: (repo: string, hash: string) => invoke<CommitDetail>('commit_show', { repo, hash }),
	commitFileDiff: (
		repo: string,
		hash: string,
		parent: string | null,
		path: string,
		oldPath: string | null
	) => invoke<string>('commit_file_diff', { repo, hash, parent, path, oldPath }),
	lfsInfo: (repo: string) => invoke<LfsInfo>('lfs_info', { repo }),
	git: (repo: string, args: string[], stdin?: string, onLine?: (line: string) => void) => {
		record('info', `git ${args.join(' ')}`);
		return invoke<string>('git_action', { repo, args, stdin, progress: lineChannel(onLine) });
	},
	clone: (url: string, dest: string, onLine?: (line: string) => void) => {
		record('info', `git clone ${url} ${dest}`);
		return invoke<string>('clone_repo', { url, dest, progress: lineChannel(onLine) });
	},
	reposIn: (path: string) => invoke<string[]>('repos_in', { path }),
	trustRepo: (path: string) => invoke<void>('trust_repo', { path }),
	ignorePaths: (repo: string, patterns: string[], paths: string[]) =>
		invoke<string[]>('ignore_paths', { repo, patterns, paths }),
	githubBuiltIn: () => invoke<boolean>('github_built_in'),
	githubLogin: (clientId: string, clientSecret: string, donePage: string) =>
		invoke<void>('github_login', { clientId, clientSecret, donePage }),
	githubLoginCancel: () => invoke<void>('github_login_cancel'),
	githubUser: () => invoke<GithubUser | null>('github_user'),
	githubLogout: () => invoke<void>('github_logout'),
	githubRepos: () => invoke<GithubRepo[]>('github_repos'),
	githubAddSshKey: (title: string, key: string) =>
		invoke<void>('github_add_ssh_key', { title, key }),
	systemAccent: () => invoke<string | null>('system_accent'),
	sshKeys: () => invoke<SshKey[]>('ssh_keys'),
	sshGenerate: (name: string, comment: string) => invoke<void>('ssh_generate', { name, comment }),
	sshTest: () => invoke<string>('ssh_test'),
	terminalOpen: (id: number, cwd: string, cols: number, rows: number, output: Channel<number[]>) =>
		invoke<void>('terminal_open', { id, cwd, cols, rows, output }),
	terminalWrite: (id: number, data: string) => invoke<void>('terminal_write', { id, data }),
	terminalResize: (id: number, cols: number, rows: number) =>
		invoke<void>('terminal_resize', { id, cols, rows }),
	terminalClose: (id: number) => invoke<void>('terminal_close', { id })
};
