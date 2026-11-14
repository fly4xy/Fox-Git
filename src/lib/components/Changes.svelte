<script lang="ts">
	import Splitter from './Splitter.svelte';
	import { Minus, Plus, Undo2 } from '@lucide/svelte';
	import { api, type FileChange } from '$lib/api';
	import { t } from '$lib/i18n.svelte';
	import { app, ignoreChoices, type Repo } from '$lib/state.svelte';
	import ConflictView from './ConflictView.svelte';
	import DiffView from './DiffView.svelte';
	import FileList from './FileList.svelte';
	import ViewToggle from './ViewToggle.svelte';

	let { repo }: { repo: Repo } = $props();

	const none = new Set<string>();

	let patch = $state('');
	let selection = $state.raw({ staged: false, paths: none });

	const status = $derived(repo.status);

	$effect(() => {
		repo.revision;
		const target = repo.file;
		if (!target || target.conflict) {
			patch = '';
			return;
		}
		api.fileDiff(repo.path, target.path, target.staged, target.untracked).then(
			(text) => {
				if (target === repo.file) patch = text;
			},
			(error) => app.toast(String(error))
		);
	});

	const selectedIn = (staged: boolean) => (selection.staged === staged ? selection.paths : none);

	function select(staged: boolean, paths: Set<string>, focus?: FileChange) {
		selection = { staged, paths };
		if (focus) repo.file = { path: focus.path, staged, untracked: focus.code === '?' };
	}

	function selectAll(staged: boolean) {
		const files = staged ? status.staged : status.unstaged;
		select(staged, new Set(files.map((f) => f.path)));
	}

	/** A row action applies to the whole selection when the row is part of it. */
	function targets(file: FileChange, staged: boolean) {
		const files = staged ? status.staged : status.unstaged;
		const paths = selectedIn(staged);
		return paths.has(file.path) ? files.filter((f) => paths.has(f.path)) : [file];
	}

	function fileMenu(event: MouseEvent, files: FileChange[], staged: boolean) {
		const count = files.length > 1 ? ` (${files.length})` : '';
		app.showMenu(event, [
			staged
				? { label: t('changes.unstage') + count, action: () => repo.unstage(files) }
				: { label: t('changes.stage') + count, action: () => repo.stage(files) },
			...(files.length === 1
				? [{ label: t('common.copyPath'), action: () => app.copy(files[0].path) }]
				: []),
			...ignoreChoices(files.map((f) => f.path)).map((choice, i) => ({
				label: t('ignore.pattern', { pattern: choice.label }),
				divider: i === 0,
				action: () => repo.ignore(choice.patterns)
			})),
			{
				label: t('changes.revertFiles') + count,
				danger: true,
				divider: true,
				action: () => repo.revert(files, staged)
			}
		]);
	}

	function resolve(file: FileChange, side: 'ours' | 'theirs') {
		repo.run(['checkout', `--${side}`, '--', file.path]).then((ok) => ok && repo.stage([file]));
	}

	const hunkActions = $derived.by(() => {
		const file = repo.file;
		if (!file || file.untracked) return [];
		if (file.staged) {
			return [
				{
					label: t('changes.hunkUnstage'),
					run: (p: string) => repo.run(['apply', '--cached', '--reverse', '-'], undefined, p)
				}
			];
		}
		return [
			{
				label: t('changes.hunkRevert'),
				danger: true,
				run: (p: string) => repo.run(['apply', '--reverse', '-'], undefined, p)
			},
			{
				label: t('changes.hunkStage'),
				run: (p: string) => repo.run(['apply', '--cached', '-'], undefined, p)
			}
		];
	});

	const canCommit = $derived(
		repo.summary.trim().length > 0 &&
			(status.staged.length > 0 || repo.amend || status.state === 'merge') &&
			status.conflicted.length === 0
	);

	let descriptionBox = $state<HTMLTextAreaElement>();

	function onMessageKey(event: KeyboardEvent) {
		if (event.key === 'Enter' && (event.ctrlKey || event.metaKey) && canCommit) repo.commit();
	}

	// A summary is one line: Enter moves on to the description.
	function onSummaryKey(event: KeyboardEvent) {
		if (event.key !== 'Enter' || event.ctrlKey || event.metaKey) return onMessageKey(event);
		event.preventDefault();
		descriptionBox?.focus();
	}

	// A pasted message splits into its first line and the rest.
	function onSummaryPaste(event: ClipboardEvent) {
		const text = event.clipboardData?.getData('text') ?? '';
		if (!text.includes('\n') || repo.description.trim()) return;
		event.preventDefault();
		const [first, ...rest] = text.replace(/\r\n/g, '\n').trim().split('\n');
		repo.summary = first;
		repo.description = rest.join('\n').trim();
		descriptionBox?.focus();
	}

	function onWindowKey(event: KeyboardEvent) {
		if (event.key.toLowerCase() !== 'a' || !(event.ctrlKey || event.metaKey)) return;
		const target = event.target as HTMLElement;
		if (target.closest('input, textarea') || app.dialog) return;
		event.preventDefault();
		selectAll(selection.staged);
	}

	let changesBox = $state<HTMLElement>();
	let unstagedBox = $state<HTMLElement>();
	let stagedBox = $state<HTMLElement>();
</script>

<svelte:window onkeydown={onWindowKey} />

<div class="changes" bind:this={changesBox}>
	<div class="side" style:width="{app.settings.changesWidth}px">
		{#if status.state}
			<div class="banner">
				<span class="grow">{t(`changes.${status.state}` as 'changes.merge')}</span>
				<button class="btn ghost" onclick={() => repo.abortOperation()}>{t('changes.abort')}</button>
				<button class="btn" disabled={status.conflicted.length > 0} onclick={() => repo.continueOperation()}>
					{t('changes.continue')}
				</button>
			</div>
		{/if}

		{#if status.conflicted.length}
			<section class="group fixed">
				<header>
					<span class="grow">{t('changes.conflicts')}</span>
					<span class="mono muted">{status.conflicted.length}</span>
				</header>
				<ul>
					{#each status.conflicted as file (file.path)}
						<li class:active={repo.file?.conflict && repo.file.path === file.path}>
							<span class="code U">U</span>
							<button
								class="pick ellipsis grow"
								title={file.path}
								onclick={() => (repo.file = { path: file.path, staged: false, untracked: false, conflict: true })}
							>
								{file.path}
							</button>
							<button class="btn ghost small" onclick={() => resolve(file, 'ours')}>{t('changes.ours')}</button>
							<button class="btn ghost small" onclick={() => resolve(file, 'theirs')}>{t('changes.theirs')}</button>
						</li>
					{/each}
				</ul>
			</section>
		{/if}

		<section class="group" bind:this={unstagedBox} style:flex-grow={app.settings.unstagedShare}>
			<header>
				<span class="grow ellipsis">{t('changes.unstaged')}</span>
				<span class="mono muted">{status.unstaged.length}</span>
				<ViewToggle />
				<button class="btn ghost small" disabled={!status.unstaged.length} onclick={() => selectAll(false)}>
					{t('changes.selectAll')}
				</button>
				<button class="btn ghost small" disabled={!status.unstaged.length} onclick={() => repo.stage(status.unstaged)}>
					{t('changes.stageAll')}
				</button>
			</header>
			<FileList
				{repo}
				files={status.unstaged}
				selected={selectedIn(false)}
				onselect={(paths, focus) => select(false, paths, focus)}
				onopen={(file) => repo.stage([file])}
				onmenu={(event, files) => fileMenu(event, files, false)}
			>
				{#snippet actions(file)}
					<button
						class="icon-btn"
						title={t('changes.revertFiles')}
						aria-label={t('changes.revertFiles')}
						onclick={() => repo.revert(targets(file, false), false)}
					>
						<Undo2 size={16} strokeWidth={1.5} />
					</button>
					<button
						class="icon-btn"
						title={t('changes.stage')}
						aria-label={t('changes.stage')}
						onclick={() => repo.stage(targets(file, false))}
					>
						<Plus size={16} strokeWidth={1.5} />
					</button>
				{/snippet}
			</FileList>
		</section>

		<Splitter
			axis="y"
			bind:value={app.settings.unstagedShare}
			min={0.15}
			max={0.85}
			span={() => unstagedBox!.offsetHeight + stagedBox!.offsetHeight}
		/>

		<section class="group" bind:this={stagedBox} style:flex-grow={1 - app.settings.unstagedShare}>
			<header>
				<span class="grow ellipsis">{t('changes.staged')}</span>
				<span class="mono muted">{status.staged.length}</span>
				<button class="btn ghost small" disabled={!status.staged.length} onclick={() => selectAll(true)}>
					{t('changes.selectAll')}
				</button>
				<button class="btn ghost small" disabled={!status.staged.length} onclick={() => repo.unstage(status.staged)}>
					{t('changes.unstageAll')}
				</button>
			</header>
			<FileList
				{repo}
				files={status.staged}
				selected={selectedIn(true)}
				onselect={(paths, focus) => select(true, paths, focus)}
				onopen={(file) => repo.unstage([file])}
				onmenu={(event, files) => fileMenu(event, files, true)}
			>
				{#snippet actions(file)}
					<button
						class="icon-btn"
						title={t('changes.unstage')}
						aria-label={t('changes.unstage')}
						onclick={() => repo.unstage(targets(file, true))}
					>
						<Minus size={16} strokeWidth={1.5} />
					</button>
				{/snippet}
			</FileList>
		</section>

		<form
			class="commit"
			onsubmit={(e) => {
				e.preventDefault();
				repo.commit();
			}}
		>
			<input
				class="input"
				placeholder={t('changes.summary')}
				aria-label={t('changes.summary')}
				bind:value={repo.summary}
				onkeydown={onSummaryKey}
				onpaste={onSummaryPaste}
			/>
			<textarea
				placeholder={t('changes.description')}
				aria-label={t('changes.description')}
				bind:this={descriptionBox}
				bind:value={repo.description}
				onkeydown={onMessageKey}
			></textarea>
			<div class="commit-row">
				<label class="check secondary">
					<input type="checkbox" checked={repo.amend} onchange={(e) => repo.toggleAmend(e.currentTarget.checked)} />
					{t('changes.amend')}
				</label>
				<button class="btn primary" type="submit" disabled={!canCommit}>{t('changes.commit')}</button>
			</div>
		</form>
	</div>

	<Splitter
		axis="x"
		bind:value={app.settings.changesWidth}
		min={260}
		max={() => Math.max(260, (changesBox?.offsetWidth ?? 0) - 320)}
	/>

	<div class="main">
		{#if repo.file?.conflict}
			<ConflictView {repo} path={repo.file.path} />
		{:else if repo.file}
			<DiffView {patch} path={repo.file.path} actions={hunkActions} />
		{:else}
			<p class="empty muted">{repo.changeCount ? t('changes.pickFile') : t('changes.clean')}</p>
		{/if}
	</div>
</div>

<style>
	.changes {
		display: flex;
		height: 100%;
	}

	.side {
		display: flex;
		flex-direction: column;
		flex: none;
		max-width: calc(100% - 320px);
		border-right: 1px solid var(--line);
	}

	.main {
		flex: 1;
		min-width: 0;
		background: var(--card);
	}

	.banner {
		display: flex;
		align-items: center;
		flex: none;
		gap: 8px;
		padding: 8px 8px 8px 16px;
		border-bottom: 1px solid var(--line);
		color: var(--accent-ink);
	}

	.group {
		display: flex;
		flex-direction: column;
		flex: 1 1 0;
		min-height: 0;
		border-bottom: 1px solid var(--line);
	}

	.group.fixed {
		flex: none;
		max-height: 30%;
	}

	.group header {
		display: flex;
		align-items: center;
		flex: none;
		gap: 8px;
		height: 36px;
		padding: 0 8px 0 16px;
		color: var(--text-2);
	}

	.grow {
		flex: 1;
		min-width: 0;
	}

	.small {
		height: 24px;
		padding: 0 8px;
		font-size: 12px;
		border-radius: 6px;
	}

	.group ul {
		flex: 1;
		min-height: 0;
		margin: 0;
		padding: 0 8px 8px;
		overflow-y: auto;
		list-style: none;
	}

	.group li {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 28px;
		padding: 0 4px 0 8px;
		border-radius: 6px;
		color: var(--text-2);
	}

	.group li.active {
		background: var(--raised);
		color: var(--text);
	}

	.pick {
		height: 100%;
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		text-align: left;
		cursor: pointer;
	}

	.commit {
		display: flex;
		flex-direction: column;
		flex: none;
		gap: 8px;
		padding: 12px;
	}

	textarea {
		height: 72px;
		padding: 8px 12px;
		border: 1px solid var(--line);
		border-radius: 8px;
		background: var(--raised);
		outline: none;
		resize: none;
		user-select: text;
		transition: border-color 140ms var(--ease);
	}

	.commit .input:focus,
	textarea:focus {
		border-color: var(--accent);
	}

	.commit-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}

	.empty {
		padding: 32px;
		text-align: center;
	}
</style>
