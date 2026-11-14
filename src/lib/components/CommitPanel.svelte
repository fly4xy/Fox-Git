<script lang="ts">
	import { api, type CommitDetail, type FileChange } from '$lib/api';
	import { t } from '$lib/i18n.svelte';
	import { app, formatTime, type Repo } from '$lib/state.svelte';
	import DiffView from './DiffView.svelte';
	import FileList from './FileList.svelte';
	import ViewToggle from './ViewToggle.svelte';

	let { repo, hash }: { repo: Repo; hash: string } = $props();

	let detail = $state<CommitDetail | null>(null);
	let file = $state<FileChange | null>(null);
	let patch = $state('');

	const selected = $derived(new Set(file ? [file.path] : []));

	$effect(() => {
		const target = hash;
		api.commitShow(repo.path, target).then(
			(loaded) => {
				if (target !== hash) return;
				detail = loaded;
				file = loaded.files[0] ?? null;
			},
			(error) => app.toast(String(error))
		);
	});

	$effect(() => {
		if (!detail || !file) {
			patch = '';
			return;
		}
		const target = file;
		api
			.commitFileDiff(repo.path, detail.hash, detail.parents[0] ?? null, target.path, target.old_path)
			.then(
				(text) => {
					if (target === file) patch = text;
				},
				(error) => app.toast(String(error))
			);
	});

	function fileMenu(event: MouseEvent, files: FileChange[]) {
		app.showMenu(event, [{ label: t('common.copyPath'), action: () => app.copy(files[0].path) }]);
	}
</script>

<div class="panel">
	{#if detail}
		<section class="info">
			<div class="meta">
				<p class="message selectable">{detail.message}</p>
				<p class="secondary">
					{detail.author}
					<span class="muted">{formatTime(detail.time)}</span>
				</p>
				<div class="line">
					<button class="hash mono muted" title={t('commit.copyHash')} onclick={() => app.copy(detail!.hash)}>
						{detail.hash.slice(0, 12)}
					</button>
					<ViewToggle />
				</div>
			</div>
			<div class="files">
				<FileList
					{repo}
					files={detail.files}
					{selected}
					onselect={(_, focus) => {
						if (focus) file = focus;
					}}
					onmenu={fileMenu}
				/>
			</div>
		</section>
		<section class="diff">
			{#if file}
				<DiffView {patch} path={file.path} />
			{:else}
				<p class="muted none">{t('commit.noFiles')}</p>
			{/if}
		</section>
	{/if}
</div>

<style>
	.panel {
		display: flex;
		height: 100%;
		background: var(--card);
	}

	.info {
		display: flex;
		flex-direction: column;
		flex: none;
		width: 320px;
		border-right: 1px solid var(--line);
	}

	.meta {
		display: flex;
		flex-direction: column;
		flex: none;
		gap: 4px;
		max-height: 45%;
		padding: 12px 16px 8px;
		overflow-y: auto;
		border-bottom: 1px solid var(--line);
	}

	.message {
		font-weight: 500;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.line {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.hash {
		padding: 0;
		border: 0;
		background: none;
		cursor: pointer;
	}

	.hash:hover {
		color: var(--text);
	}

	.files {
		display: flex;
		flex: 1;
		flex-direction: column;
		min-height: 0;
		padding-top: 8px;
	}

	.diff {
		flex: 1;
		min-width: 0;
	}

	.none {
		padding: 24px;
	}
</style>
