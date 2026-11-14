<script lang="ts">
	import { app, type Repo } from '$lib/state.svelte';
	import CommitPanel from './CommitPanel.svelte';
	import GraphList from './GraphList.svelte';

	let { repo }: { repo: Repo } = $props();

	let root = $state<HTMLElement>();
	function startResize(event: PointerEvent) {
		const target = event.currentTarget as HTMLElement;
		target.setPointerCapture(event.pointerId);
		const move = (e: PointerEvent) => {
			const box = root!.getBoundingClientRect();
			app.settings.historyShare = Math.min(0.75, Math.max(0.2, (box.bottom - e.clientY) / box.height));
		};
		target.addEventListener('pointermove', move);
		target.addEventListener(
			'pointerup',
			() => {
				target.removeEventListener('pointermove', move);
				app.save();
			},
			{ once: true }
		);
	}
</script>

<div class="history" bind:this={root}>
	<div class="graph"><GraphList {repo} /></div>
	{#if repo.selected}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="handle" onpointerdown={startResize}></div>
		<div class="detail" style:height="{app.settings.historyShare * 100}%">
			<CommitPanel {repo} hash={repo.selected} />
		</div>
	{/if}
</div>

<style>
	.history {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.graph {
		flex: 1;
		min-height: 0;
	}

	.handle {
		flex: none;
		height: 5px;
		margin-bottom: -4px;
		border-bottom: 1px solid var(--line);
		position: relative;
		z-index: 1;
		cursor: row-resize;
		transition: border-color 140ms var(--ease);
	}

	.handle:hover {
		border-color: var(--accent);
	}

	.detail {
		flex: none;
		min-height: 0;
	}
</style>
