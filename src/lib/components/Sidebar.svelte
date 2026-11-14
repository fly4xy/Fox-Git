<script lang="ts">
	import {
		Archive,
		ChevronDown,
		ChevronRight,
		Cloud,
		FileBox,
		GitBranch,
		GitCommitVertical,
		PencilLine,
		Tag
	} from '@lucide/svelte';
	import { t } from '$lib/i18n.svelte';
	import { app, type Repo } from '$lib/state.svelte';

	let { repo }: { repo: Repo } = $props();

	let filter = $state('');
	let closed = $state<Record<string, boolean>>({ remotes: true, tags: true });

	const overview = $derived(repo.overview);
	const match = (name: string) => name.toLowerCase().includes(filter.toLowerCase());

	const branches = $derived(overview?.branches.filter((b) => match(b.name)) ?? []);
	const remotes = $derived(overview?.remote_branches.filter(match) ?? []);
	const tags = $derived(overview?.tags.filter(match) ?? []);
	const stashes = $derived(overview?.stashes.filter((s) => match(s.message)) ?? []);

	function showHash(hash: string) {
		repo.view = 'history';
		repo.selected = hash;
	}

	function tagMenu(event: MouseEvent, name: string) {
		app.showMenu(event, [
			{ label: t('tag.checkout'), action: () => repo.run(['checkout', name]) },
			{ label: t('common.copyName'), action: () => app.copy(name) },
			{ label: t('common.delete'), danger: true, action: () => repo.run(['tag', '-d', name]) }
		]);
	}

	function stashMenu(event: MouseEvent, name: string) {
		app.showMenu(event, [
			{ label: t('stash.apply'), action: () => repo.run(['stash', 'apply', name], t('busy.stash')) },
			{ label: t('stash.pop'), action: () => repo.run(['stash', 'pop', name], t('busy.stash')) },
			{ label: t('common.delete'), danger: true, action: () => repo.stashDrop(name) }
		]);
	}
</script>

{#snippet section(id: string, title: string, count: number)}
	<button class="section" onclick={() => (closed[id] = !closed[id])}>
		{#if closed[id]}
			<ChevronRight size={14} strokeWidth={1.5} />
		{:else}
			<ChevronDown size={14} strokeWidth={1.5} />
		{/if}
		<span>{title}</span>
		<span class="mono count">{count}</span>
	</button>
{/snippet}

<aside style:width="{app.settings.sidebarWidth}px">
	<nav class="views">
		<button class="row" class:active={repo.view === 'changes'} onclick={() => (repo.view = 'changes')}>
			<PencilLine size={16} strokeWidth={1.5} />
			<span class="grow">{t('side.changes')}</span>
			{#if repo.changeCount}<span class="mono changes">{repo.changeCount}</span>{/if}
		</button>
		<button class="row" class:active={repo.view === 'history'} onclick={() => (repo.view = 'history')}>
			<GitCommitVertical size={16} strokeWidth={1.5} />
			<span class="grow">{t('side.history')}</span>
		</button>
		<button
			class="row"
			class:active={repo.view === 'lfs'}
			onclick={() => {
				repo.view = 'lfs';
				repo.loadLfs().catch((error) => app.toast(String(error)));
			}}
		>
			<FileBox size={16} strokeWidth={1.5} />
			<span class="grow">Git LFS</span>
		</button>
	</nav>

	<div class="filter"><input class="input" placeholder={t('common.filter')} bind:value={filter} /></div>

	<div class="tree">
		{@render section('branches', t('side.branches'), branches.length)}
		{#if !closed.branches}
			{#each branches as branch (branch.name)}
				<button
					class="row"
					class:current={branch.current}
					title={branch.upstream || t('side.noUpstream')}
					ondblclick={() => !branch.current && !branch.elsewhere && repo.checkout(branch.name)}
					onclick={() => showHash(branch.hash)}
					oncontextmenu={(e) => app.showMenu(e, repo.branchItems(branch))}
				>
					<GitBranch size={16} strokeWidth={1.5} />
					<span class="grow ellipsis">{branch.name}</span>
					{#if branch.ahead}<span class="mono muted">↑{branch.ahead}</span>{/if}
					{#if branch.behind}<span class="mono muted">↓{branch.behind}</span>{/if}
				</button>
			{/each}
		{/if}

		{@render section('remotes', t('side.remotes'), remotes.length)}
		{#if !closed.remotes}
			{#each remotes as name (name)}
				<button class="row" oncontextmenu={(e) => app.showMenu(e, repo.remoteBranchItems(name))}>
					<Cloud size={16} strokeWidth={1.5} />
					<span class="grow ellipsis">{name}</span>
				</button>
			{/each}
		{/if}

		{@render section('tags', t('side.tags'), tags.length)}
		{#if !closed.tags}
			{#each tags as name (name)}
				<button class="row" oncontextmenu={(e) => tagMenu(e, name)}>
					<Tag size={16} strokeWidth={1.5} />
					<span class="grow ellipsis">{name}</span>
				</button>
			{/each}
		{/if}

		{@render section('stashes', t('side.stashes'), stashes.length)}
		{#if !closed.stashes}
			{#each stashes as stash (stash.hash)}
				<button
					class="row"
					title={stash.message}
					onclick={() => showHash(stash.hash)}
					oncontextmenu={(e) => stashMenu(e, stash.name)}
				>
					<Archive size={16} strokeWidth={1.5} />
					<span class="grow ellipsis">{stash.message}</span>
				</button>
			{/each}
		{/if}
	</div>
</aside>

<style>
	aside {
		display: flex;
		flex-direction: column;
		flex: none;
		max-width: 40vw;
		border-right: 1px solid var(--line);
		background: var(--deep);
	}

	.views {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 8px;
	}

	.filter {
		padding: 0 8px 8px;
	}

	.tree {
		flex: 1;
		min-height: 0;
		padding: 0 8px 12px;
		overflow-y: auto;
	}

	.row,
	.section {
		display: flex;
		align-items: center;
		width: 100%;
		border: 0;
		background: none;
		text-align: left;
		cursor: pointer;
	}

	.row {
		gap: 8px;
		height: 32px;
		padding: 0 8px;
		border-radius: 8px;
		color: var(--text-2);
		transition:
			background-color 140ms var(--ease),
			color 140ms var(--ease);
	}

	.row :global(svg) {
		flex: none;
		color: var(--text-3);
	}

	.row:hover {
		background: var(--hover);
	}

	.row.active {
		background: var(--raised);
		color: var(--text);
		box-shadow: inset 0 1px 0 var(--line);
	}

	.row.current {
		color: var(--text);
		font-weight: 500;
	}

	.row.current :global(svg) {
		color: var(--accent-ink);
	}

	.grow {
		flex: 1;
		min-width: 0;
	}

	.changes {
		color: var(--accent-ink);
	}

	.section {
		gap: 4px;
		height: 24px;
		margin-top: 12px;
		margin-bottom: 4px;
		padding: 0 4px;
		color: var(--text-3);
	}

	.section:hover {
		color: var(--text-2);
	}

	.count {
		margin-left: auto;
		padding-right: 4px;
	}
</style>
