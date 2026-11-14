<script lang="ts">
	import { parseDiff, type Hunk } from '$lib/diff';
	import { t } from '$lib/i18n.svelte';

	const MAX_LINES = 6000;

	interface HunkAction {
		label: string;
		danger?: boolean;
		run: (patch: string) => void;
	}

	let {
		patch,
		path,
		actions = []
	}: { patch: string; path: string; actions?: HunkAction[] } = $props();

	const diff = $derived(parseDiff(patch));
	const shown = $derived.by(() => {
		let budget = MAX_LINES;
		const hunks: Hunk[] = [];
		for (const hunk of diff.hunks) {
			if (budget <= 0) break;
			hunks.push(budget >= hunk.lines.length ? hunk : { ...hunk, lines: hunk.lines.slice(0, budget) });
			budget -= hunk.lines.length;
		}
		return { hunks, cut: budget < 0 || hunks.length < diff.hunks.length };
	});
</script>

<div class="diff">
	<header class="path mono secondary ellipsis" title={path}>{path}</header>
	<div class="body">
		{#if diff.binary}
			<p class="note muted">{t('diff.binary')}</p>
		{:else if !diff.hunks.length}
			<p class="note muted">{t('diff.noText')}</p>
		{/if}
		{#each shown.hunks as hunk}
			<section>
				{#if hunk.title || actions.length}
					<div class="hunk-head">
						<span class="mono muted ellipsis">{hunk.title}</span>
						{#each actions as action}
							<button class="hunk-btn" class:danger={action.danger} onclick={() => action.run(diff.header + hunk.raw)}>
								{action.label}
							</button>
						{/each}
					</div>
				{/if}
				{#each hunk.lines as line}
					<div class="line {line.kind}">
						<span class="no">{line.oldNo ?? ''}</span>
						<span class="no">{line.newNo ?? ''}</span>
						<span class="text selectable">{line.text}</span>
					</div>
				{/each}
			</section>
		{/each}
		{#if shown.cut}
			<p class="note muted">{t('diff.limit', { n: MAX_LINES })}</p>
		{/if}
	</div>
</div>

<style>
	.diff {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.path {
		flex: none;
		padding: 8px 16px;
		border-bottom: 1px solid var(--line);
	}

	.body {
		flex: 1;
		min-height: 0;
		overflow: auto;
	}

	section {
		width: max-content;
		min-width: 100%;
	}

	.hunk-head {
		display: flex;
		align-items: center;
		gap: 8px;
		position: sticky;
		left: 0;
		width: 100%;
		max-width: 100vw;
		height: 32px;
		padding: 0 8px 0 16px;
		background: var(--raised);
	}

	.hunk-head .ellipsis {
		flex: 1;
	}

	.hunk-btn {
		flex: none;
		height: 24px;
		padding: 0 8px;
		border: 1px solid var(--line);
		border-radius: 6px;
		background: var(--card);
		color: var(--text-2);
		font-size: 12px;
		line-height: 16px;
		cursor: pointer;
		transition: color 140ms var(--ease);
	}

	.hunk-btn:hover {
		color: var(--text);
	}

	.hunk-btn.danger:hover {
		color: var(--neg);
	}

	.line {
		display: flex;
		font: 400 12px/20px var(--mono);
	}

	.line.add {
		background: color-mix(in srgb, var(--pos) 13%, transparent);
	}

	.line.del {
		background: color-mix(in srgb, var(--neg) 13%, transparent);
	}

	.line.note {
		color: var(--text-3);
	}

	.no {
		flex: none;
		width: 48px;
		padding-right: 8px;
		color: var(--text-3);
		text-align: right;
		font-variant-numeric: tabular-nums;
	}

	.text {
		padding: 0 16px 0 8px;
		white-space: pre;
		tab-size: 4;
	}

	.note {
		padding: 16px;
	}
</style>
