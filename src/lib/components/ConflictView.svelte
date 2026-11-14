<script lang="ts">
	import { api, type Conflict, type ConflictSide } from '$lib/api';
	import { locale, t } from '$lib/i18n.svelte';
	import { app, type Repo } from '$lib/state.svelte';
	import DiffView from './DiffView.svelte';

	let { repo, path }: { repo: Repo; path: string } = $props();

	let conflict = $state<Conflict | null>(null);

	$effect(() => {
		repo.revision;
		const target = path;
		conflict = null;
		api.conflictDiff(repo.path, target).then(
			(result) => {
				if (target === path) conflict = result;
			},
			(error) => app.toast(String(error))
		);
	});

	const size = (bytes: number) =>
		bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toLocaleString(locale(), { maximumFractionDigits: 1 })} KB`;
	const date = (time: number) =>
		new Date(time * 1000).toLocaleString(locale(), { dateStyle: 'medium', timeStyle: 'short' });
</script>

{#snippet side(title: string, mark: string, info: ConflictSide)}
	<div class="side">
		<div class="title"><span class="mono mark">{mark}</span>{title}</div>
		{#if info.size === null}
			<div class="muted">{t('conflict.deleted')}</div>
		{:else}
			<div class="mono secondary">{size(info.size)}{info.time ? ` · ${date(info.time)}` : ''}</div>
		{/if}
		{#if info.commit}<div class="mono muted ellipsis" title={info.commit}>{info.commit}</div>{/if}
	</div>
{/snippet}

<div class="conflict">
	{#if conflict}
		<div class="sides">
			{@render side(t('conflict.ours', { name: repo.overview?.head ?? 'HEAD' }), '−', conflict.ours)}
			{@render side(t('conflict.theirs', { name: conflict.theirs.name }), '+', conflict.theirs)}
		</div>
		{#if conflict.diff}
			<div class="diff"><DiffView patch={conflict.diff} {path} /></div>
		{:else}
			<p class="empty muted">
				{conflict.ours.size !== null && conflict.theirs.size !== null ? t('conflict.same') : t('conflict.oneDeleted')}
			</p>
		{/if}
	{/if}
</div>

<style>
	.conflict {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.sides {
		display: grid;
		grid-template-columns: 1fr 1fr;
		flex: none;
		border-bottom: 1px solid var(--line);
	}

	.side {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
		padding: 12px 16px;
	}

	.side + .side {
		border-left: 1px solid var(--line);
	}

	.title {
		display: flex;
		gap: 8px;
		color: var(--text);
	}

	.mark {
		color: var(--text-3);
	}

	.diff {
		flex: 1;
		min-height: 0;
	}

	.empty {
		padding: 32px;
		text-align: center;
	}
</style>
