<script lang="ts">
	import '@fontsource-variable/geist';
	import '@fontsource-variable/geist-mono';
	import '../app.css';
	import { onMount } from 'svelte';
	import { app } from '$lib/state.svelte';
	import Changes from '$lib/components/Changes.svelte';
	import CloneDialog from '$lib/components/CloneDialog.svelte';
	import Console from '$lib/components/Console.svelte';
	import Dialog from '$lib/components/Dialog.svelte';
	import History from '$lib/components/History.svelte';
	import Lfs from '$lib/components/Lfs.svelte';
	import LogPanel from '$lib/components/LogPanel.svelte';
	import Menu from '$lib/components/Menu.svelte';
	import Settings from '$lib/components/Settings.svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import Splitter from '$lib/components/Splitter.svelte';
	import Toasts from '$lib/components/Toasts.svelte';
	import Toolbar from '$lib/components/Toolbar.svelte';
	import Welcome from '$lib/components/Welcome.svelte';

	let ready = $state(false);

	onMount(async () => {
		await app.init();
		ready = true;
	});
</script>

{#if ready}
	<div class="shell">
		<Toolbar />
		<Console />
		{#if app.active}
			{@const repo = app.active}
			<div class="body">
				<Sidebar {repo} />
				<Splitter
					axis="x"
					bind:value={app.settings.sidebarWidth}
					min={180}
					max={() => Math.min(480, window.innerWidth * 0.4)}
				/>
				<main>
					{#if repo.view === 'history'}
						<History {repo} />
					{:else if repo.view === 'changes'}
						<Changes {repo} />
					{:else}
						<Lfs {repo} />
					{/if}
				</main>
			</div>
		{:else}
			<Welcome />
		{/if}
		<LogPanel />
	</div>

	{#if app.settingsOpen}<Settings />{/if}
	{#if app.cloneOpen}<CloneDialog />{/if}
	{#if app.dialog}<Dialog request={app.dialog} />{/if}
	{#if app.menu}<Menu menu={app.menu} />{/if}
	<Toasts />
{/if}

<style>
	.shell {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}

	.body {
		display: flex;
		flex: 1;
		min-height: 0;
	}

	main {
		flex: 1;
		min-width: 0;
		background: var(--bg);
	}
</style>
