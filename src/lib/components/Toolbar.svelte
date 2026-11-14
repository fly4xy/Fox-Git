<script lang="ts">
	import {
		ArrowDown,
		ArrowUp,
		Archive,
		ChevronLeft,
		ChevronRight,
		Copy,
		GitBranchPlus,
		GitMerge,
		Minus,
		Plus,
		RefreshCw,
		ScrollText,
		Settings,
		Square,
		SquareTerminal,
		X
	} from '@lucide/svelte';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { onMount } from 'svelte';
	import { log } from '$lib/log.svelte';
	import { t } from '$lib/i18n.svelte';
	import { app } from '$lib/state.svelte';
	import FoxMark from './FoxMark.svelte';
	import Progress from './Progress.svelte';

	const repo = $derived(app.active);
	const branch = $derived(repo?.currentBranch ?? null);

	const index = $derived(repo ? app.tabs.indexOf(repo) : -1);
	let strip: HTMLElement | undefined = $state();

	// Keep the active tab visible when the strip overflows.
	$effect(() => {
		if (index >= 0) strip?.children[index]?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
	});

	const win = getCurrentWindow();
	let maximized = $state(false);

	onMount(() => {
		const sync = () => win.isMaximized().then((value) => (maximized = value));
		sync();
		const off = win.onResized(sync);
		return () => off.then((stop) => stop());
	});
</script>

<!-- Empty parts of the toolbar move the window, as a title bar would. -->
<header data-tauri-drag-region>
	<div class="brand" data-tauri-drag-region><FoxMark /></div>

	<Progress />
	<div class="spacer" data-tauri-drag-region></div>

	{#if repo}
		<div class="actions">
			<button class="btn ghost" disabled={app.working} onclick={() => repo.fetch()}>
				<RefreshCw size={16} strokeWidth={1.5} />Fetch
			</button>
			<button class="btn ghost" disabled={app.working} onclick={() => repo.pull()}>
				<ArrowDown size={16} strokeWidth={1.5} />Pull
				{#if branch?.behind}<span class="mono">{branch.behind}</span>{/if}
			</button>
			<button class="btn ghost" disabled={app.working} onclick={() => repo.push()}>
				<ArrowUp size={16} strokeWidth={1.5} />Push
				{#if branch?.ahead}<span class="mono">{branch.ahead}</span>{/if}
			</button>
			<span class="divider"></span>
			<button class="btn ghost" onclick={() => repo.newBranch()}>
				<GitBranchPlus size={16} strokeWidth={1.5} />{t('toolbar.branch')}
			</button>
			<button class="btn ghost" disabled={app.working || !!repo.status.state} onclick={() => repo.mergeOrRebase()}>
				<GitMerge size={16} strokeWidth={1.5} />{t('toolbar.merge')}
			</button>
			<button class="btn ghost" onclick={() => repo.stashPush()}>
				<Archive size={16} strokeWidth={1.5} />{t('toolbar.stash')}
			</button>
			<span class="divider"></span>
			<button
				class="icon-btn square"
				class:on={app.terminalOpen}
				aria-label={t('toolbar.terminal')}
				aria-pressed={app.terminalOpen}
				title={t('toolbar.terminal')}
				onclick={() => (app.terminalOpen = !app.terminalOpen)}
			>
				<SquareTerminal size={16} strokeWidth={1.5} />
			</button>
		</div>
	{/if}

	<button
		class="icon-btn square"
		class:on={log.open}
		aria-label={t('toolbar.log')}
		aria-pressed={log.open}
		title={t('toolbar.log')}
		onclick={() => (log.open = !log.open)}
	>
		<ScrollText size={16} strokeWidth={1.5} />
	</button>
	<button class="icon-btn square" aria-label={t('toolbar.settings')} onclick={() => (app.settingsOpen = true)}>
		<Settings size={16} strokeWidth={1.5} />
	</button>

	{#if !app.settings.systemTitleBar}
		<div class="window">
			<button class="icon-btn square" aria-label={t('window.minimize')} title={t('window.minimize')} onclick={() => win.minimize()}>
				<Minus size={16} strokeWidth={1.5} />
			</button>
			<button
				class="icon-btn square"
				aria-label={maximized ? t('window.restore') : t('window.maximize')}
				title={maximized ? t('window.restore') : t('window.maximize')}
				onclick={() => win.toggleMaximize()}
			>
				{#if maximized}<Copy size={14} strokeWidth={1.5} />{:else}<Square size={14} strokeWidth={1.5} />{/if}
			</button>
			<button class="icon-btn square close-window" aria-label={t('common.close')} title={t('common.close')} onclick={() => win.close()}>
				<X size={16} strokeWidth={1.5} />
			</button>
		</div>
	{/if}
</header>

<nav class="tabbar" data-tauri-drag-region>
	<button
		class="icon-btn square"
		aria-label={t('toolbar.prevTab')}
		title={t('toolbar.prevTab')}
		disabled={index <= 0}
		onclick={() => app.selectTab(app.tabs[index - 1])}
	>
		<ChevronLeft size={16} strokeWidth={1.5} />
	</button>
	<div class="tabs" bind:this={strip}>
		{#each app.tabs as tab (tab.path)}
			<div class="tab" class:active={tab === app.active}>
				<button class="tab-name ellipsis" title={tab.path} onclick={() => app.selectTab(tab)}>
					{tab.name}
				</button>
				<button class="icon-btn close" aria-label={t('toolbar.closeTab')} onclick={() => app.closeTab(tab)}>
					<X size={14} strokeWidth={1.5} />
				</button>
			</div>
		{/each}
	</div>
	<button
		class="icon-btn square"
		aria-label={t('toolbar.nextTab')}
		title={t('toolbar.nextTab')}
		disabled={index >= app.tabs.length - 1}
		onclick={() => app.selectTab(app.tabs[index + 1])}
	>
		<ChevronRight size={16} strokeWidth={1.5} />
	</button>
	<button class="icon-btn square" aria-label={t('toolbar.newTab')} title={t('toolbar.newTab')} onclick={() => (app.active = null)}>
		<Plus size={16} strokeWidth={1.5} />
	</button>
</nav>

<style>
	header {
		display: flex;
		align-items: center;
		flex: none;
		gap: 12px;
		height: 48px;
		padding: 0 12px;
		border-bottom: 1px solid var(--line);
		background: var(--deep);
	}

	.brand {
		display: flex;
		flex: none;
		padding: 0 4px;
	}

	.spacer {
		flex: 1;
		align-self: stretch;
	}

	.tabbar {
		display: flex;
		align-items: center;
		flex: none;
		gap: 4px;
		height: 40px;
		padding: 0 8px;
		border-bottom: 1px solid var(--line);
		background: var(--deep);
	}

	.tabbar .icon-btn:disabled {
		opacity: 0.35;
		cursor: default;
		background: none;
	}

	.tabs {
		display: flex;
		align-items: center;
		flex: 1;
		gap: 4px;
		min-width: 0;
		overflow-x: auto;
		scrollbar-width: none;
	}

	.tab {
		display: flex;
		align-items: center;
		flex: 0 1 180px;
		min-width: 120px;
		height: 32px;
		padding-right: 4px;
		border-radius: 8px;
		color: var(--text-2);
		transition: background-color 140ms var(--ease);
	}

	.tab:hover {
		background: var(--hover);
	}

	.tab.active {
		background: var(--raised);
		color: var(--text);
	}

	.tab-name {
		flex: 1;
		height: 100%;
		padding: 0 4px 0 12px;
		border: 0;
		background: none;
		text-align: left;
		cursor: pointer;
	}

	.close {
		opacity: 0;
	}

	.tab:hover .close,
	.tab.active .close,
	.close:focus-visible {
		opacity: 1;
	}

	.actions {
		display: flex;
		align-items: center;
		flex: none;
		gap: 4px;
	}

	.divider {
		width: 1px;
		height: 16px;
		margin: 0 4px;
		background: var(--line-strong);
	}

	.square {
		width: 32px;
		height: 32px;
		border-radius: 8px;
	}

	.on {
		background: var(--raised);
		color: var(--text);
	}

	.window {
		display: flex;
		flex: none;
		gap: 4px;
		padding-left: 8px;
		border-left: 1px solid var(--line);
	}

	.close-window:hover {
		background: var(--neg);
		color: #fff;
	}
</style>
