<script lang="ts">
	import { app, type MenuItem } from '$lib/state.svelte';

	let { menu }: { menu: { x: number; y: number; items: MenuItem[] } } = $props();

	let width = $state(0);
	let height = $state(0);

	const left = $derived(Math.max(8, Math.min(menu.x, window.innerWidth - width - 8)));
	const top = $derived(Math.max(8, Math.min(menu.y, window.innerHeight - height - 8)));

	const close = () => (app.menu = null);
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && close()} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="layer"
	onpointerdown={close}
	oncontextmenu={(e) => {
		e.preventDefault();
		close();
	}}
></div>
<div
	class="menu"
	role="menu"
	style:left="{left}px"
	style:top="{top}px"
	bind:clientWidth={width}
	bind:clientHeight={height}
>
	{#each menu.items as item}
		{#if item.divider}<hr />{/if}
		<button
			role="menuitem"
			class:danger={item.danger}
			disabled={item.disabled}
			onclick={() => {
				close();
				item.action();
			}}
		>
			{item.label}
		</button>
	{/each}
</div>

<style>
	.layer {
		position: fixed;
		inset: 0;
		z-index: 30;
	}

	.menu {
		position: fixed;
		z-index: 31;
		display: flex;
		flex-direction: column;
		min-width: 200px;
		padding: 4px;
		border: 1px solid var(--line-strong);
		border-radius: 10px;
		background: var(--raised);
		box-shadow: 0 8px 24px rgb(0 0 0 / 0.24);
		animation: open 160ms var(--ease);
	}

	button {
		height: 28px;
		padding: 0 8px;
		border: 0;
		border-radius: 6px;
		background: none;
		text-align: left;
		white-space: nowrap;
		cursor: pointer;
	}

	button:hover:enabled {
		background: var(--hover);
	}

	button:disabled {
		opacity: 0.4;
		cursor: default;
	}

	hr {
		width: 100%;
		height: 1px;
		margin: 4px 0;
		border: 0;
		background: var(--line);
	}

	.danger {
		color: var(--neg);
	}

	@keyframes open {
		from {
			opacity: 0;
			transform: translateY(-4px);
		}
	}
</style>
