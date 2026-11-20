<script lang="ts">
	import { api, type FileCommit } from '$lib/api';
	import { t } from '$lib/i18n.svelte';
	import { app, formatTime, type Repo } from '$lib/state.svelte';
	import DiffView from './DiffView.svelte';
	import Modal from './Modal.svelte';

	let { repo, path }: { repo: Repo; path: string } = $props();

	let commits = $state<FileCommit[] | null>(null);
	let picked = $state<FileCommit | null>(null);
	let patch = $state('');

	$effect(() => {
		api.fileLog(repo.path, path).then(
			(loaded) => {
				commits = loaded;
				picked = loaded[0] ?? null;
			},
			(error) => app.toast(String(error))
		);
	});

	$effect(() => {
		const target = picked;
		patch = '';
		if (!target) return;
		api
			.commitFileDiff(repo.path, target.hash, target.parents[0] ?? null, target.path, target.old_path)
			.then(
				(text) => {
					if (target === picked) patch = text;
				},
				(error) => app.toast(String(error))
			);
	});

	const name = $derived(path.split('/').pop() ?? path);
</script>

<Modal title={t('history.title', { path: name })} width={1100} onclose={() => (app.history = null)}>
	<div class="history">
		<ul class="commits">
			{#each commits ?? [] as commit (commit.hash)}
				<li>
					<button class="commit" class:active={picked === commit} title={commit.hash} onclick={() => (picked = commit)}>
						<span class="ellipsis">{commit.subject}</span>
						<span class="meta secondary">
							<span class="ellipsis">{commit.author}</span>
							<span class="mono muted">{formatTime(commit.time)} · {commit.hash.slice(0, 7)}</span>
						</span>
						{#if commit.old_path}
							<span class="muted ellipsis">{t('history.renamed', { path: commit.old_path })}</span>
						{/if}
					</button>
				</li>
			{/each}
			{#if commits && !commits.length}<p class="muted empty">{t('history.empty')}</p>{/if}
		</ul>
		<section class="diff">
			{#if picked}<DiffView {patch} path={picked.path} />{/if}
		</section>
	</div>
</Modal>

<style>
	.history {
		display: flex;
		height: min(640px, calc(100vh - 160px));
		margin: 0 -20px -20px;
		border-top: 1px solid var(--line);
	}

	.commits {
		flex: none;
		width: 320px;
		margin: 0;
		padding: 8px;
		overflow-y: auto;
		list-style: none;
		border-right: 1px solid var(--line);
	}

	.commit {
		display: flex;
		flex-direction: column;
		gap: 2px;
		width: 100%;
		min-width: 0;
		padding: 8px 12px;
		border: 0;
		border-radius: 8px;
		background: none;
		color: var(--text);
		text-align: left;
		cursor: pointer;
	}

	.commit:hover {
		background: var(--hover);
	}

	.commit.active {
		background: var(--raised);
	}

	.meta {
		display: flex;
		justify-content: space-between;
		gap: 8px;
		min-width: 0;
		font-size: 12px;
	}

	.meta .mono {
		flex: none;
	}

	.diff {
		flex: 1;
		min-width: 0;
		overflow: auto;
	}

	.empty {
		padding: 24px 12px;
		text-align: center;
	}
</style>
