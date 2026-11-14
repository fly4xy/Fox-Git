<script lang="ts">
	import { X } from '@lucide/svelte';
	import { t } from '$lib/i18n.svelte';
	import { log, type LogLevel } from '$lib/log.svelte';
	import { app } from '$lib/state.svelte';
	import Splitter from './Splitter.svelte';

	let filter = $state<LogLevel | null>(null);
	let list = $state<HTMLElement>();

	const shown = $derived(filter ? log.entries.filter((e) => e.level === filter) : log.entries);
	const count = (level: LogLevel) => log.entries.filter((e) => e.level === level).length;
	const filters = $derived([
		{ level: null, label: t('log.all'), n: log.entries.length },
		{ level: 'error' as const, label: t('log.errors'), n: count('error') },
		{ level: 'warning' as const, label: t('log.warnings'), n: count('warning') }
	]);

	// The newest entry stays in view.
	$effect(() => {
		if (shown.length && list) list.scrollTop = list.scrollHeight;
	});
</script>

{#if log.open}
	<!-- A negative span makes dragging up grow the panel below the border. -->
	<Splitter
		axis="y"
		bind:value={app.settings.logHeight}
		min={96}
		max={() => window.innerHeight * 0.6}
		span={() => -1}
	/>
	<section style:height="{app.settings.logHeight}px" aria-label={t('log.title')}>
		<header>
			<span class="title">{t('log.title')}</span>
			{#each filters as item (item.level)}
				<button
					class="btn ghost small"
					class:on={filter === item.level}
					aria-pressed={filter === item.level}
					onclick={() => (filter = item.level)}
				>
					{item.label}<span class="mono muted">{item.n}</span>
				</button>
			{/each}
			<span class="grow"></span>
			<button class="btn ghost small" disabled={!log.entries.length} onclick={() => (log.entries = [])}>
				{t('log.clear')}
			</button>
			<button class="icon-btn" aria-label={t('common.close')} title={t('common.close')} onclick={() => (log.open = false)}>
				<X size={16} strokeWidth={1.5} />
			</button>
		</header>
		<ol bind:this={list}>
			{#each shown as entry (entry.id)}
				<li class={entry.level}>
					<span class="mono muted">{new Date(entry.time).toLocaleTimeString()}</span>
					<span class="text mono selectable">{entry.text}</span>
				</li>
			{:else}
				<li class="muted">{t('log.empty')}</li>
			{/each}
		</ol>
	</section>
{/if}

<style>
	section {
		display: flex;
		flex-direction: column;
		flex: none;
		border-top: 1px solid var(--line);
		background: var(--deep);
	}

	header {
		display: flex;
		align-items: center;
		flex: none;
		gap: 4px;
		height: 36px;
		padding: 0 8px 0 16px;
		border-bottom: 1px solid var(--line);
	}

	.title {
		margin-right: 8px;
	}

	.btn.small {
		gap: 6px;
	}

	.on {
		background: var(--raised);
		color: var(--text);
	}

	ol {
		flex: 1;
		min-height: 0;
		margin: 0;
		padding: 8px 16px;
		overflow: auto;
		list-style: none;
	}

	li {
		display: flex;
		gap: 12px;
		padding: 2px 0;
		line-height: 20px;
	}

	li > :first-child {
		flex: none;
	}

	.text {
		min-width: 0;
		white-space: pre-wrap;
		word-break: break-word;
	}

	.error .text {
		color: var(--neg);
	}

	.warning .text {
		color: var(--warn);
	}
</style>
