<script lang="ts">
	import { app } from '$lib/state.svelte';
	import TerminalPane from './TerminalPane.svelte';

	const MIN_HEIGHT = 120;

	const shown = $derived(app.terminalOpen && !!app.active);
	const sessions = $derived(app.tabs.filter((tab) => tab.terminal));

	// The shell of a repository starts the first time the panel shows that repository.
	$effect(() => {
		if (shown && app.active) app.active.terminal = true;
	});

	function resize(event: PointerEvent) {
		const handle = event.currentTarget as HTMLElement;
		const start = event.clientY - app.settings.terminalHeight;
		const move = (e: PointerEvent) => {
			const limit = window.innerHeight * 0.7;
			app.settings.terminalHeight = Math.round(Math.max(MIN_HEIGHT, Math.min(limit, e.clientY - start)));
		};
		const stop = () => {
			handle.removeEventListener('pointermove', move);
			app.save();
		};
		handle.setPointerCapture(event.pointerId);
		handle.addEventListener('pointermove', move);
		handle.addEventListener('pointerup', stop, { once: true });
	}
</script>

<section class:shown style:height="{app.settings.terminalHeight}px">
	{#each sessions as tab (tab.path)}
		<TerminalPane repo={tab} visible={shown && tab === app.active} />
	{/each}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="handle" onpointerdown={resize}></div>
</section>

<style>
	section {
		position: relative;
		display: none;
		flex: none;
		border-bottom: 1px solid var(--line);
		background: var(--deep);
	}

	section.shown {
		display: block;
	}

	.handle {
		position: absolute;
		right: 0;
		bottom: -4px;
		left: 0;
		z-index: 1;
		height: 8px;
		cursor: row-resize;
	}
</style>
