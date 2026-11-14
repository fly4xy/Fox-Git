<script lang="ts">
	import { Check, LoaderCircle } from '@lucide/svelte';
	import { t } from '$lib/i18n.svelte';
	import { app } from '$lib/state.svelte';

	const task = $derived(app.task);
</script>

{#if task}
	<div class="track" class:done={task.done} aria-hidden="true">
		{#if task.percent === null}
			<span class="bar sweep"></span>
		{:else}
			<span class="bar" style:width="{task.percent}%"></span>
		{/if}
	</div>

	<div class="status" class:done={task.done} role="status" title={task.detail || task.label}>
		{#if task.done}
			<Check size={16} strokeWidth={1.5} />
		{:else}
			<LoaderCircle class="spin" size={16} strokeWidth={1.5} />
		{/if}
		<span class="label ellipsis">{task.label}</span>
		{#if task.done}
			<span class="detail muted">{t('busy.done')}</span>
		{:else if task.detail}
			<span class="detail mono muted ellipsis">{task.detail}</span>
		{/if}
	</div>
{/if}

<style>
	/* Both appear after a short delay, so instant commands do not flash. */
	.track {
		position: fixed;
		top: 0;
		right: 0;
		left: 0;
		z-index: 50;
		height: 2px;
		overflow: hidden;
		animation: appear 160ms var(--ease) 120ms both;
	}

	.bar {
		display: block;
		height: 100%;
		background: var(--accent);
		transition: width 240ms var(--ease);
	}

	.sweep {
		width: 32%;
		animation: sweep 1.2s var(--ease) infinite;
	}

	.track.done {
		animation: fade 400ms var(--ease) 800ms both;
	}

	.status {
		display: flex;
		align-items: center;
		flex: 0 1 auto;
		gap: 8px;
		min-width: 0;
		max-width: 360px;
		height: 32px;
		padding: 0 12px;
		border-radius: 8px;
		background: var(--raised);
		color: var(--text-2);
		animation: appear 160ms var(--ease) 120ms both;
	}

	.status :global(svg) {
		flex: none;
		color: var(--text-3);
	}

	.status.done :global(svg) {
		color: var(--accent-ink);
	}

	.label {
		flex: none;
		max-width: 200px;
		color: var(--text);
	}

	.detail {
		min-width: 0;
	}

	.status :global(.spin) {
		animation: spin 900ms linear infinite;
	}

	@keyframes appear {
		from {
			opacity: 0;
		}
	}

	@keyframes fade {
		to {
			opacity: 0;
		}
	}

	@keyframes sweep {
		from {
			transform: translateX(-100%);
		}
		to {
			transform: translateX(320%);
		}
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.sweep {
			width: 100%;
			animation: none;
			opacity: 0.6;
		}

		.status :global(.spin) {
			animation: none;
		}
	}
</style>
