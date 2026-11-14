import { open } from '@tauri-apps/plugin-dialog';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { record } from './log.svelte';
import { SvelteSet } from 'svelte/reactivity';
import {
	api,
	type Branch,
	type Commit,
	type FileChange,
	type GithubUser,
	type LfsInfo,
	type Overview,
	type Status
} from './api';
import { layoutGraph, type GraphLayout } from './graph';
import { locale, setLanguage, t, type Language } from './i18n.svelte';
import { applyTheme, onSystemThemeChange, readSystemAccent, type Mode } from './theme';

const PAGE = 2000;
const PATHS_PER_COMMAND = 200;
const SETTINGS_KEY = 'fox-git.settings';
const AUTH_FAILURES = [
	'Authentication failed',
	'could not read Username',
	'terminal prompts disabled',
	'Permission denied (publickey)'
];

interface Settings {
	palette: string;
	mode: Mode;
	lang: Language;
	fileView: 'list' | 'tree';
	terminalHeight: number;
	logHeight: number;
	/** The title bar of the system instead of the window buttons in the toolbar. */
	systemTitleBar: boolean;
	sidebarWidth: number;
	changesWidth: number;
	/** The part of the change lists that the unstaged files take. */
	unstagedShare: number;
	/** The part of the history view that the selected commit takes. */
	historyShare: number;
	clientId: string;
	recent: string[];
	tabs: string[];
	active: string | null;
}

function loadSettings(): Settings {
	const defaults: Settings = {
		palette: 'fox',
		mode: 'system',
		lang: 'system',
		fileView: 'list',
		terminalHeight: 320,
		logHeight: 200,
		systemTitleBar: false,
		sidebarWidth: 256,
		changesWidth: 392,
		unstagedShare: 0.5,
		historyShare: 0.42,
		clientId: '',
		recent: [],
		tabs: [],
		active: null
	};
	try {
		return { ...defaults, ...JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? '{}') };
	} catch {
		return defaults;
	}
}

function describeError(error: unknown) {
	const text = String(error);
	return AUTH_FAILURES.some((failure) => text.includes(failure))
		? `${text}\n\n${t('auth.hint')}`
		: text;
}

/** Splits paths so that a long selection fits the command line limit. */
function batches(command: string[], paths: string[]) {
	const commands: string[][] = [];
	for (let i = 0; i < paths.length; i += PATHS_PER_COMMAND) {
		commands.push([...command, '--', ...paths.slice(i, i + PATHS_PER_COMMAND)]);
	}
	return commands;
}

export interface DialogField {
	key: string;
	label: string;
	type?: 'text' | 'checkbox' | 'select';
	value?: string | boolean;
	/** A function makes the list depend on the other fields. */
	options?: string[] | ((values: Record<string, any>) => string[]);
}

export interface DialogRequest {
	title: string;
	message?: string;
	fields?: DialogField[];
	confirm: string;
	danger?: boolean;
	resolve: (values: Record<string, any> | null) => void;
}

export interface Task {
	label: string;
	/** The last line git printed, such as `Receiving objects: 45% (9/20)`. */
	detail: string;
	/** Empty while the length of the work is unknown. */
	percent: number | null;
	done: boolean;
}

/** Takes the step and percentage out of a git progress line. */
function follow(task: Task, line: string) {
	task.detail = line.replace(/^remote:\s*/, '');
	const percent = /(\d+)%/.exec(line);
	if (percent) task.percent = Number(percent[1]);
}

/** Names a command that came without a label, as in `git checkout main`. */
function describe(args: string[]) {
	const end = args.indexOf('--');
	return ['git', ...args.slice(0, end === -1 ? 3 : Math.min(end, 3))].join(' ');
}

/** A `.gitignore` line that matches exactly this path from the repository root. */
function ignorePattern(path: string) {
	return '/' + path.replace(/[\\*?[]/g, '\\$&').replace(/ $/, '\\ ');
}

/** Choices for ignoring files: the files, their extension, every folder they share. */
export function ignoreChoices(paths: string[]) {
	const choices: { label: string; patterns: string[] }[] = [];
	if (paths.length === 1) {
		const pattern = ignorePattern(paths[0]);
		choices.push({ label: pattern, patterns: [pattern] });
	} else {
		choices.push({ label: t('ignore.files', { n: paths.length }), patterns: paths.map(ignorePattern) });
	}
	const extensions = new Set(paths.map((path) => /\.([^./]+)$/.exec(path)?.[1] ?? ''));
	const [extension] = extensions;
	if (extensions.size === 1 && extension) {
		const pattern = `*.${extension.replace(/[\\*?[]/g, '\\$&')}`;
		choices.push({ label: pattern, patterns: [pattern] });
	}
	const folders = paths.map((path) => path.split('/').slice(0, -1));
	const shared = folders[0].filter((part, i) => folders.every((folder) => folder[i] === part));
	for (let depth = shared.length; depth > 0; depth--) {
		const pattern = ignorePattern(shared.slice(0, depth).join('/')) + '/';
		choices.push({ label: pattern, patterns: [pattern] });
	}
	return choices;
}

export interface MenuItem {
	label: string;
	danger?: boolean;
	disabled?: boolean;
	/** Draws a line above the item. */
	divider?: boolean;
	action: () => void;
}

export interface FileSelection {
	path: string;
	staged: boolean;
	untracked: boolean;
	/** A file with merge conflicts: both versions are compared. */
	conflict?: boolean;
}

export class Repo {
	path: string;
	name: string;
	overview = $state<Overview | null>(null);
	status = $state<Status>({ staged: [], unstaged: [], conflicted: [], state: null, message: null });
	commits = $state.raw<Commit[]>([]);
	graph = $state.raw<GraphLayout>({ rows: [], width: 1 });
	lfs = $state<LfsInfo | null>(null);
	view = $state<'history' | 'changes' | 'lfs'>('history');
	selected = $state<string | null>(null);
	file = $state<FileSelection | null>(null);
	summary = $state('');
	description = $state('');
	amend = $state(false);
	collapsed = new SvelteSet<string>();
	/** Stays true while the shell of this repository is alive, also when the panel is hidden. */
	terminal = $state(false);
	/** Grows on each refresh so open diffs reload. */
	revision = $state(0);
	exhausted = false;

	constructor(path: string) {
		this.path = path;
		this.name = path.split(/[\\/]/).filter(Boolean).pop() ?? path;
	}

	get changeCount() {
		const { staged, unstaged, conflicted } = this.status;
		return new Set([...staged, ...unstaged, ...conflicted].map((f) => f.path)).size;
	}

	get currentBranch() {
		return this.overview?.branches.find((b) => b.current) ?? null;
	}

	async refresh() {
		const limit = Math.max(PAGE, this.commits.length);
		const [overview, status, commits] = await Promise.all([
			api.overview(this.path),
			api.status(this.path),
			api.log(this.path, limit, 0)
		]);
		this.overview = overview;
		this.status = status;
		this.fillPreparedMessage();
		this.followFile();
		this.setCommits(commits);
		this.exhausted = commits.length < limit;
		this.revision++;
		if (this.view === 'lfs') await this.loadLfs();
	}

	/** The message git prepared for this merge, filled in once, so it can still be cleared. */
	private preparedMessage: string | null = null;

	private fillPreparedMessage() {
		const message = this.status.message;
		if (message && message !== this.preparedMessage && !this.summary.trim() && !this.description.trim()) {
			const [first, ...rest] = message.split('\n');
			this.summary = first;
			this.description = rest.join('\n').trim();
		}
		this.preparedMessage = message;
	}

	/** Keeps the open diff on the file after it moves in or out of the index. */
	private followFile() {
		const file = this.file;
		if (!file) return;
		const inList = (list: FileChange[]) => list.some((f) => f.path === file.path);
		if (file.conflict) {
			if (inList(this.status.conflicted)) return;
			this.file = null;
			return;
		}
		const { staged, unstaged } = this.status;
		if (inList(file.staged ? staged : unstaged)) return;
		if (inList(file.staged ? unstaged : staged)) {
			this.file = { path: file.path, staged: !file.staged, untracked: false };
		} else {
			this.file = null;
		}
	}

	async loadMore() {
		if (this.exhausted) return;
		this.exhausted = true;
		const more = await api.log(this.path, PAGE, this.commits.length);
		this.setCommits([...this.commits, ...more]);
		this.exhausted = more.length < PAGE;
	}

	async loadLfs() {
		this.lfs = await api.lfsInfo(this.path);
	}

	private setCommits(commits: Commit[]) {
		this.commits = commits;
		this.graph = layoutGraph(commits);
	}

	/** Runs git commands in order, reports a failure and refreshes the repository. */
	private async runAll(commands: string[][], label?: string, stdin?: string) {
		try {
			await app.track(label ?? describe(commands[0]), !!label, async (task) => {
				for (const [i, args] of commands.entries()) {
					if (commands.length > 1) task.percent = (i / commands.length) * 100;
					await api.git(this.path, args, stdin, (line) => follow(task, line));
				}
			});
			return true;
		} catch (error) {
			app.toast(describeError(error));
			return false;
		} finally {
			await this.refresh().catch((error) => app.toast(String(error)));
		}
	}

	run(args: string[], label?: string, stdin?: string) {
		return this.runAll([args], label, stdin);
	}

	stage = (files: FileChange[]) =>
		this.runAll(
			batches(
				['add', '-A'],
				files.map((f) => f.path)
			)
		);

	unstage = (files: FileChange[]) =>
		this.runAll(
			batches(
				this.overview?.head_hash ? ['reset', '-q', 'HEAD'] : ['rm', '--cached'],
				files.map((f) => f.path)
			)
		);

	private revertUnstaged(files: FileChange[]) {
		const paths = (fresh: boolean) =>
			files.filter((f) => (f.code === '?') === fresh).map((f) => f.path);
		return [...batches(['clean', '-f'], paths(true)), ...batches(['restore'], paths(false))];
	}

	private revertStaged(files: FileChange[]) {
		const noHistory = !this.overview?.head_hash;
		const isNew = (f: FileChange) => noHistory || f.code === 'A' || f.code === 'R';
		const created = files.filter(isNew).map((f) => f.path);
		const restored = [
			...files.filter((f) => !isNew(f)).map((f) => f.path),
			...files.flatMap((f) => (f.old_path ? [f.old_path] : []))
		];
		return [
			...batches(['rm', '-f'], created),
			...batches(['restore', '--source=HEAD', '--staged', '--worktree'], restored)
		];
	}

	async revert(files: FileChange[], staged: boolean) {
		const ok = await app.ask({
			title: t('changes.revertTitle'),
			message:
				files.length === 1
					? t('changes.revertOne', { path: files[0].path })
					: t('changes.revertMany', { n: files.length }),
			confirm: t('changes.revertFiles'),
			danger: true
		});
		if (!ok) return;
		const commands = staged ? this.revertStaged(files) : this.revertUnstaged(files);
		await this.runAll(commands, t('busy.revert', { n: files.length }));
	}

	/** Writes the patterns, then drops the files they cover from the change lists. */
	async ignore(patterns: string[]) {
		const { staged, unstaged } = this.status;
		const listed = [...staged, ...unstaged];
		let matched: Set<string>;
		try {
			const paths = [...new Set(listed.map((f) => f.path))];
			const found = await app.track(t('busy.ignore'), false, () =>
				api.ignorePaths(this.path, patterns, paths)
			);
			matched = new Set(found);
		} catch (error) {
			app.toast(String(error));
			return;
		}
		// New files leave the index quietly: they were never committed.
		const added = staged.filter((f) => matched.has(f.path) && f.code === 'A').map((f) => f.path);
		const loose = new Set([...added, ...unstaged.filter((f) => f.code === '?').map((f) => f.path)]);
		const tracked = [...new Set(listed.map((f) => f.path))].filter(
			(path) => matched.has(path) && !loose.has(path)
		);
		const commands = batches(['rm', '--cached', '-q', '-f'], added);
		if (tracked.length) {
			const ok = await app.ask({
				title: t('ignore.trackedTitle'),
				message: t('ignore.trackedMessage', { n: tracked.length }),
				confirm: t('ignore.untrack')
			});
			if (ok) commands.push(...batches(['rm', '--cached', '-q', '-f'], tracked));
		}
		this.file = null;
		if (commands.length) await this.runAll(commands, t('busy.ignore'));
		else await this.refresh().catch((error) => app.toast(String(error)));
	}

	/** The message that amending filled in, so turning it off can take it back. */
	private amended: { summary: string; description: string } | null = null;

	/** Amending starts from the message of the last commit, unless something is typed already. */
	async toggleAmend(on: boolean) {
		this.amend = on;
		if (!on) {
			const filled = this.amended;
			if (filled?.summary === this.summary && filled.description === this.description) {
				this.summary = '';
				this.description = '';
			}
			this.amended = null;
			return;
		}
		const head = this.overview?.head_hash;
		if (!head || this.summary.trim() || this.description.trim()) return;
		try {
			const { message } = await api.commitShow(this.path, head);
			const [first, ...rest] = message.trim().split('\n');
			this.summary = first;
			this.description = rest.join('\n').trim();
			this.amended = { summary: this.summary, description: this.description };
		} catch (error) {
			app.toast(String(error));
		}
	}

	async commit() {
		const description = this.description.trim();
		const done = await this.run(
			[
				'commit',
				...(this.amend ? ['--amend'] : []),
				'-m',
				this.summary.trim(),
				...(description ? ['-m', description] : [])
			],
			t('busy.commit')
		);
		if (done) {
			this.summary = '';
			this.description = '';
			this.amend = false;
			this.amended = null;
			this.file = null;
		}
	}

	fetch = () => this.run(['fetch', '--all', '--prune'], t('busy.fetch'));

	checkout = (name: string) => this.run(['checkout', name], t('busy.checkout', { name }));

	/** Splits `origin/feature/x` by the real remote name, which may contain a slash too. */
	private remoteRef(ref: string) {
		const remotes = [...(this.overview?.remotes ?? [])].sort((a, b) => b.length - a.length);
		const remote = remotes.find((name) => ref.startsWith(`${name}/`));
		return remote ? { remote, branch: ref.slice(remote.length + 1) } : null;
	}

	async pull() {
		const branch = this.currentBranch;
		const remotes = this.overview?.remotes ?? [];
		if (!branch || !remotes.length) {
			app.toast(t('pull.nothing'), 'info');
			return;
		}
		const remote = this.remoteRef(branch.upstream)?.remote ?? remotes[0];
		const values = await app.ask({
			title: 'Pull',
			message: t('pull.into', { name: branch.name }),
			fields: [
				{ key: 'remote', label: t('pull.remote'), type: 'select', options: remotes, value: remote },
				{
					key: 'branch',
					label: t('pull.branch'),
					type: 'select',
					options: (v) =>
						this.overview?.remote_branches.filter((name) => name.startsWith(`${v.remote}/`)) ?? [],
					value: branch.upstream || `${remote}/${branch.name}`
				},
				{ key: 'rebase', label: t('pull.rebase'), type: 'checkbox' },
				{ key: 'stash', label: t('pull.stash'), type: 'checkbox' }
			],
			confirm: 'Pull'
		});
		if (!values?.branch) return;
		const args = ['pull', values.rebase ? '--rebase' : '--no-rebase'];
		if (values.stash) args.push('--autostash');
		args.push(values.remote, values.branch.slice(values.remote.length + 1));
		await this.run(args, t('busy.pullFrom', { source: values.branch, branch: branch.name }));
	}

	async push(name?: string) {
		const branches = this.overview?.branches ?? [];
		const remotes = this.overview?.remotes ?? [];
		if (!branches.length || !remotes.length) {
			app.toast(t('push.nothing'), 'info');
			return;
		}
		const upstreamOf = (branch: string) => branches.find((b) => b.name === branch)?.upstream ?? '';
		const targets = (branch: string) => {
			const upstream = upstreamOf(branch);
			const known = this.remoteRef(upstream) ? [upstream] : [];
			return [...new Set([...known, ...remotes.map((remote) => `${remote}/${branch}`)])];
		};
		const start = name ?? this.currentBranch?.name ?? branches[0].name;
		const values = await app.ask({
			title: 'Push',
			fields: [
				{
					key: 'branch',
					label: t('push.branch'),
					type: 'select',
					options: branches.map((b) => b.name),
					value: start
				},
				{
					key: 'target',
					label: t('push.to'),
					type: 'select',
					options: (v) => targets(v.branch),
					value: targets(start)[0]
				},
				{ key: 'tags', label: t('push.tags'), type: 'checkbox' },
				{ key: 'force', label: t('push.force'), type: 'checkbox' }
			],
			confirm: 'Push'
		});
		const target = values && this.remoteRef(values.target);
		if (!target) return;
		const args = ['push'];
		if (!upstreamOf(values.branch)) args.push('-u');
		if (values.tags) args.push('--tags');
		if (values.force) args.push('--force-with-lease');
		args.push(target.remote, `refs/heads/${values.branch}:refs/heads/${target.branch}`);
		await this.run(args, t('busy.pushTo', { branch: values.branch, target: values.target }));
	}

	async newBranch(start?: string) {
		const overview = this.overview;
		const head = overview?.head ?? 'HEAD';
		const from = start ?? head;
		const sources = [
			from,
			head,
			...(overview?.branches.map((b) => b.name) ?? []),
			...(overview?.remote_branches ?? []),
			...(overview?.tags ?? [])
		];
		const values = await app.ask({
			title: t('branch.newTitle'),
			fields: [
				{ key: 'name', label: t('branch.name') },
				{
					key: 'from',
					label: t('branch.from'),
					type: 'select',
					options: [...new Set(sources)],
					value: from
				},
				{ key: 'checkout', label: t('branch.switch'), type: 'checkbox', value: true }
			],
			confirm: t('common.create')
		});
		if (!values?.name) return;
		// A branch made from origin/x must not silently track origin/x.
		const source = overview?.head_hash ? ['--no-track', values.from] : [];
		await this.run(
			values.checkout
				? ['checkout', '-b', values.name, ...source]
				: ['branch', values.name, ...source]
		);
	}

	async renameBranch(name: string) {
		const values = await app.ask({
			title: t('branch.renameTitle'),
			fields: [{ key: 'name', label: t('branch.name'), value: name }],
			confirm: t('branch.rename')
		});
		if (values?.name && values.name !== name) await this.run(['branch', '-m', name, values.name]);
	}

	async setUpstream(branch: Branch) {
		const options = this.overview?.remote_branches ?? [];
		const values = await app.ask({
			title: t('branch.setUpstream'),
			fields: [
				{
					key: 'upstream',
					label: t('branch.upstream'),
					type: 'select',
					options,
					value: branch.upstream || options[0]
				}
			],
			confirm: t('common.select')
		});
		if (values?.upstream) {
			await this.run(['branch', `--set-upstream-to=${values.upstream}`, branch.name]);
		}
	}

	async deleteRemoteBranch(ref: string) {
		const target = this.remoteRef(ref);
		if (!target) return;
		const ok = await app.ask({
			title: t('branch.deleteTitle'),
			message: t('branch.deleteRemoteMessage', { name: ref }),
			confirm: t('common.delete'),
			danger: true
		});
		if (ok) {
			await this.run(['push', target.remote, '--delete', target.branch], t('busy.deleteRemote', { name: ref }));
		}
	}

	/** Every branch action; the ones that cannot apply right now stay visible but disabled. */
	branchItems(branch: Branch): MenuItem[] {
		const { name, current, elsewhere, upstream } = branch;
		const head = this.overview?.head ?? 'HEAD';
		const midway = !!this.status.state;
		const tracked = this.remoteRef(upstream);
		return [
			{
				label: t('branch.checkout'),
				disabled: current || elsewhere || midway,
				action: () => this.checkout(name)
			},
			{
				label: t('branch.mergeInto', { name: head }),
				disabled: current || midway,
				action: () => this.run(['merge', name], t('busy.merge', { name, head }))
			},
			{
				label: t('branch.rebaseOnto', { name: head }),
				disabled: current || midway,
				action: () => this.run(['rebase', name], t('busy.rebase', { name, head }))
			},
			{
				label: 'Pull',
				divider: true,
				disabled: !current && (!tracked || elsewhere),
				action: () =>
					current
						? this.pull()
						: tracked &&
							this.run(
								['fetch', tracked.remote, `${tracked.branch}:${name}`],
								t('busy.pullFrom', { source: upstream, branch: name })
							)
			},
			{ label: 'Push', action: () => this.push(name) },
			{ label: t('branch.newHere'), divider: true, action: () => this.newBranch(name) },
			{ label: t('graph.newTag'), action: () => this.newTag(name) },
			{ label: t('branch.rename'), action: () => this.renameBranch(name) },
			{
				label: t('branch.setUpstream'),
				disabled: !this.overview?.remote_branches.length,
				action: () => this.setUpstream(branch)
			},
			{
				label: t('branch.unsetUpstream'),
				disabled: !upstream,
				action: () => this.run(['branch', '--unset-upstream', name])
			},
			{ label: t('common.copyName'), divider: true, action: () => app.copy(name) },
			{
				label: t('common.delete'),
				danger: true,
				disabled: current || elsewhere,
				action: () => this.deleteBranch(name)
			},
			{
				label: t('branch.deleteRemote', { name: upstream || `origin/${name}` }),
				danger: true,
				disabled: !tracked,
				action: () => this.deleteRemoteBranch(upstream)
			}
		];
	}

	remoteBranchItems(name: string): MenuItem[] {
		const head = this.overview?.head ?? 'HEAD';
		const midway = !!this.status.state;
		const short = this.remoteRef(name)?.branch;
		return [
			{
				label: t('branch.track'),
				disabled: midway || !!this.overview?.branches.some((b) => b.name === short),
				action: () => this.run(['checkout', '--track', name], t('busy.checkout', { name }))
			},
			{
				label: t('branch.fetchLocal'),
				disabled: !short || !!this.overview?.branches.some((b) => b.name === short),
				action: () => this.fetchLocal(name)
			},
			{
				label: t('branch.mergeInto', { name: head }),
				disabled: midway,
				action: () => this.run(['merge', name], t('busy.merge', { name, head }))
			},
			{
				label: t('branch.rebaseOnto', { name: head }),
				disabled: midway,
				action: () => this.run(['rebase', name], t('busy.rebase', { name, head }))
			},
			{ label: t('branch.newHere'), divider: true, action: () => this.newBranch(name) },
			{ label: t('common.copyName'), divider: true, action: () => app.copy(name) },
			{
				label: t('branch.deleteRemote', { name }),
				danger: true,
				action: () => this.deleteRemoteBranch(name)
			}
		];
	}

	/** Brings a remote branch into the local ones without switching to it. */
	async fetchLocal(name: string) {
		const ref = this.remoteRef(name);
		if (!ref) return;
		await this.runAll(
			[
				['fetch', ref.remote, `refs/heads/${ref.branch}:refs/heads/${ref.branch}`],
				['branch', `--set-upstream-to=${name}`, ref.branch]
			],
			t('busy.fetchBranch', { name: ref.branch })
		);
	}

	/** One dialog for merging a branch into HEAD or rebasing HEAD onto it. */
	async mergeOrRebase() {
		const head = this.overview?.head ?? 'HEAD';
		const names = [
			...(this.overview?.branches.filter((b) => !b.current).map((b) => b.name) ?? []),
			...(this.overview?.remote_branches ?? [])
		];
		if (!names.length) return app.toast(t('merge.nothing'), 'info');
		const merge = t('branch.mergeInto', { name: head });
		const values = await app.ask({
			title: t('merge.title'),
			fields: [
				{ key: 'name', label: t('merge.branch'), type: 'select', options: names },
				{
					key: 'mode',
					label: t('merge.mode'),
					type: 'select',
					options: [merge, t('branch.rebaseOnto', { name: head })]
				},
				{ key: 'noFf', label: t('merge.noFf'), type: 'checkbox', value: false }
			],
			confirm: t('merge.run')
		});
		if (!values) return;
		const { name } = values;
		if (values.mode === merge) {
			await this.run(['merge', ...(values.noFf ? ['--no-ff'] : []), name], t('busy.merge', { name, head }));
		} else {
			await this.run(['rebase', name], t('busy.rebase', { name, head }));
		}
	}

	async newTag(hash: string) {
		const values = await app.ask({
			title: t('tag.newTitle'),
			fields: [{ key: 'name', label: t('tag.name') }],
			confirm: t('common.create')
		});
		if (values?.name) await this.run(['tag', values.name, hash]);
	}

	async deleteBranch(name: string) {
		const ok = await app.ask({
			title: t('branch.deleteTitle'),
			message: t('branch.deleteMessage', { name }),
			confirm: t('common.delete'),
			danger: true
		});
		if (ok) await this.run(['branch', '-D', name]);
	}

	async reset(hash: string, mode: 'soft' | 'mixed' | 'hard') {
		if (mode === 'hard') {
			const ok = await app.ask({
				title: t('graph.hardTitle'),
				message: t('graph.hardMessage', { hash: hash.slice(0, 8) }),
				confirm: t('graph.hardConfirm'),
				danger: true
			});
			if (!ok) return;
		}
		await this.run(['reset', `--${mode}`, hash]);
	}

	async stashPush() {
		const files = [...this.status.staged, ...this.status.unstaged];
		const paths = [...new Set(files.map((f) => f.path))];
		if (!paths.length) {
			app.toast(t('stash.nothing'), 'info');
			return;
		}
		const values = await app.ask({
			title: t('stash.title'),
			fields: [
				{ key: 'message', label: t('stash.message') },
				{ key: 'untracked', label: t('stash.untracked'), type: 'checkbox', value: true },
				...paths.slice(0, 200).map((path) => ({
					key: `file:${path}`,
					label: path,
					type: 'checkbox' as const,
					value: true
				}))
			],
			confirm: t('stash.confirm')
		});
		if (!values) return;
		const chosen = paths.filter((path) => values[`file:${path}`] ?? true);
		if (!chosen.length) return;
		const args = ['stash', 'push'];
		if (values.untracked) args.push('--include-untracked');
		if (values.message) args.push('-m', values.message);
		if (chosen.length < paths.length) args.push('--', ...chosen);
		await this.run(args, t('busy.stash'));
		this.file = null;
	}

	async stashDrop(name: string) {
		const ok = await app.ask({
			title: t('stash.dropTitle'),
			message: t('stash.dropMessage', { name }),
			confirm: t('common.delete'),
			danger: true
		});
		if (ok) await this.run(['stash', 'drop', name]);
	}

	abortOperation() {
		const state = this.status.state;
		if (state) return this.run([state, '--abort']);
	}

	continueOperation() {
		const state = this.status.state;
		if (!state) return;
		// The fields hold the prepared message, possibly edited, so they win over git's own.
		if (state === 'merge') return this.summary.trim() ? this.commit() : this.run(['commit', '--no-edit']);
		return this.run([state, '--continue']);
	}
}

class App {
	tabs = $state<Repo[]>([]);
	active = $state<Repo | null>(null);
	settings = $state<Settings>(loadSettings());
	settingsOpen = $state(false);
	terminalOpen = $state(false);
	/** Grows on each theme change so the terminal can take the new colors. */
	themeRevision = $state(0);
	cloneOpen = $state(false);
	/** The git work under way, shown at the top of the window. */
	task = $state<Task | null>(null);
	toasts = $state<{ id: number; text: string }[]>([]);
	dialog = $state<DialogRequest | null>(null);
	menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);
	user = $state<GithubUser | null>(null);
	systemAccent = $state<string | null>(null);
	private toastId = 0;

	/** Shows the work at the top while it runs; `linger` keeps the finished state in view a moment. */
	async track<T>(label: string, linger: boolean, work: (task: Task) => Promise<T>) {
		this.task = { label, detail: '', percent: null, done: false };
		const task = this.task;
		const clear = () => {
			if (this.task === task) this.task = null;
		};
		try {
			const result = await work(task);
			task.done = true;
			task.percent = 100;
			task.detail = '';
			if (linger) setTimeout(clear, 1600);
			else clear();
			return result;
		} catch (error) {
			clear();
			throw error;
		}
	}

	get working() {
		return !!this.task && !this.task.done;
	}

	async init() {
		setLanguage(this.settings.lang);
		this.applyTitleBar();
		this.systemAccent = await readSystemAccent();
		this.theme();
		onSystemThemeChange(() => this.theme());
		window.addEventListener('focus', () => this.onFocus());
		getCurrentWebview().onDragDropEvent(({ payload }) => {
			if (payload.type === 'drop') payload.paths.forEach((path) => this.openPath(path));
		});
		api.githubUser().then(
			(user) => (this.user = user),
			() => {}
		);
		const last = this.settings.active;
		for (const path of [...this.settings.tabs]) await this.openPath(path, true);
		this.active = this.tabs.find((tab) => tab.path === last) ?? this.active;
		this.save();
		const launched = await api.launchPath();
		if (launched) await this.openPath(launched);
	}

	private async onFocus() {
		const accent = await readSystemAccent();
		if (accent !== this.systemAccent) {
			this.systemAccent = accent;
			this.theme();
		}
		this.active?.refresh().catch(() => {});
	}

	theme() {
		applyTheme(this.settings.palette, this.settings.mode, this.systemAccent);
		this.themeRevision++;
	}

	save() {
		try {
			this.settings.tabs = this.tabs.map((t) => t.path);
			if (this.active) this.settings.active = this.active.path;
			localStorage.setItem(SETTINGS_KEY, JSON.stringify(this.settings));
		} catch {
			// Settings stay in memory for this session.
		}
	}

	applyTitleBar() {
		getCurrentWindow()
			.setDecorations(this.settings.systemTitleBar)
			.catch((error) => this.toast(String(error)));
	}

	toast(text: string, level: 'info' | 'error' = 'error') {
		record(level, text);
		const id = ++this.toastId;
		this.toasts.push({ id, text });
		setTimeout(() => this.dismissToast(id), 8000);
	}

	dismissToast(id: number) {
		this.toasts = this.toasts.filter((t) => t.id !== id);
	}

	ask(request: Omit<DialogRequest, 'resolve'>) {
		return new Promise<Record<string, any> | null>((resolve) => {
			this.dialog = { ...request, resolve };
		});
	}

	showMenu(event: MouseEvent, items: MenuItem[]) {
		event.preventDefault();
		this.menu = { x: event.clientX, y: event.clientY, items };
	}

	async copy(value: string) {
		await navigator.clipboard.writeText(value);
	}

	async openPath(path: string, quiet = false) {
		try {
			const root = await api.repoRoot(path);
			const same = this.tabs.find((t) => t.path === root);
			if (same) {
				this.active = same;
				return;
			}
			const repo = new Repo(root);
			await this.track(t('busy.open', { name: repo.name }), false, () => repo.refresh());
			this.tabs.push(repo);
			this.active = repo;
			this.settings.recent = [root, ...this.settings.recent.filter((r) => r !== root)].slice(0, 12);
			this.save();
		} catch (error) {
			if (!quiet) await this.recoverOpen(path, String(error));
		}
	}

	/** Folder pickers can return a parent folder, and git refuses folders of other users. */
	private async recoverOpen(path: string, error: string) {
		if (error.includes('dubious ownership')) {
			const target = /safe\.directory\s+'?([^'\n]+)/.exec(error)?.[1]?.trim() ?? path;
			const ok = await this.ask({
				title: t('open.trustTitle'),
				message: t('open.trustMessage', { path: target }),
				confirm: t('open.trust')
			});
			if (!ok) return;
			await api.trustRepo(target);
			await this.openPath(path);
		} else if (error.includes('not a git repository')) {
			const found = await api.reposIn(path).catch(() => []);
			if (found.length === 1) {
				await this.openPath(found[0]);
			} else if (found.length > 1) {
				const values = await this.ask({
					title: t('open.pickTitle'),
					fields: [
						{ key: 'repo', label: t('open.pickLabel'), type: 'select', options: found, value: found[0] }
					],
					confirm: t('open.action')
				});
				if (values) await this.openPath(values.repo);
			} else {
				this.toast(t('open.notRepo', { path }));
			}
		} else {
			this.toast(error);
		}
	}

	async openDialog() {
		const path = await open({ directory: true, title: t('open.dialog') });
		if (typeof path === 'string') await this.openPath(path);
	}

	selectTab(repo: Repo) {
		this.active = repo;
		this.save();
		repo.refresh().catch((error) => this.toast(String(error)));
	}

	closeTab(repo: Repo) {
		const index = this.tabs.indexOf(repo);
		this.tabs.splice(index, 1);
		if (this.active === repo) this.active = this.tabs[index] ?? this.tabs[index - 1] ?? null;
		this.save();
	}

	async clone(url: string, dest: string) {
		try {
			const path = await this.track(t('busy.clone', { url }), false, (task) =>
				api.clone(url, dest, (line) => follow(task, line))
			);
			await this.openPath(path);
			this.cloneOpen = false;
		} catch (error) {
			if (String(error).includes('already exists')) {
				await this.openPath(dest);
				this.cloneOpen = false;
			} else {
				this.toast(describeError(error));
			}
		}
	}
}

export const app = new App();

export function formatTime(seconds: number) {
	const date = new Date(seconds * 1000);
	const sameYear = date.getFullYear() === new Date().getFullYear();
	const day = date.toLocaleDateString(locale(), {
		day: 'numeric',
		month: 'short',
		year: sameYear ? undefined : 'numeric'
	});
	const time = date.toLocaleTimeString(locale(), { hour: '2-digit', minute: '2-digit' });
	return `${day}, ${time}`;
}
