<script lang="ts">
	import { Cloud, Tag } from '@lucide/svelte';
	import { untrack } from 'svelte';
	import type { Commit } from '$lib/api';
	import { parseRefs, type Segment, type RefLabel } from '$lib/graph';
	import { t } from '$lib/i18n.svelte';
	import { app, formatTime, type Repo } from '$lib/state.svelte';

	let { repo }: { repo: Repo } = $props();

	const ROW = 32;
	const LANE = 16;
	const PAD = 16;
	const MAX_GRAPH = 280;
	const OVERSCAN = 8;

	let scroller = $state<HTMLElement>();
	let scrollTop = $state(0);
	let height = $state(600);

	const commits = $derived(repo.commits);
	const rows = $derived(repo.graph.rows);
	const graphWidth = $derived(Math.min(MAX_GRAPH, repo.graph.width * LANE + PAD));
	const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN));
	const last = $derived(Math.min(commits.length, Math.ceil((scrollTop + height) / ROW) + OVERSCAN));
	const visible = $derived(commits.slice(first, last));
	const headHash = $derived(repo.overview?.head_hash);

	/** The graph line under the pointer, lit along with the branch name at its top. */
	let hovered = $state<number | null>(null);

	/** The branch names of each line, taken from its topmost commit that has any. */
	const lineNames = $derived.by(() => {
		const names = new Map<number, string[]>();
		const remotes = repo.overview?.remotes ?? [];
		commits.forEach((commit, index) => {
			const line = rows[index]?.line;
			if (line === undefined || names.has(line) || !commit.refs) return;
			const branches = parseRefs(commit.refs, remotes).filter((l) => l.kind !== 'tag').map((l) => l.name);
			if (branches.length) names.set(line, branches);
		});
		return names;
	});

	const laneX = (lane: number) => PAD / 2 + 4 + lane * LANE;
	const color = (index: number) => `var(--lane-${index})`;

	function path(segment: Segment, row: number) {
		const y0 = (row - first) * ROW + ROW / 2;
		const y1 = y0 + ROW;
		const x0 = laneX(segment.from);
		const x1 = laneX(segment.to);
		if (x0 === x1) return `M${x0} ${y0}V${y1}`;
		const bend = ROW * 0.62;
		return `M${x0} ${y0}C${x0} ${y0 + bend} ${x1} ${y1 - bend} ${x1} ${y1}`;
	}

	$effect(() => {
		if (last >= commits.length - OVERSCAN) repo.loadMore().catch((e) => app.toast(String(e)));
	});

	// Selecting from the sidebar scrolls the commit into view.
	$effect(() => {
		const hash = repo.selected;
		if (!hash || !scroller) return;
		const index = untrack(() => commits.findIndex((c) => c.hash === hash));
		if (index < 0) return;
		const top = index * ROW;
		const view = scroller.clientHeight;
		if (top < scroller.scrollTop || top + ROW > scroller.scrollTop + view) {
			scroller.scrollTop = Math.max(0, top - view / 3);
		}
	});

	function refMenu(event: MouseEvent, label: RefLabel) {
		if (label.kind === 'tag') return;
		const branch = repo.overview?.branches.find((b) => b.name === label.name);
		event.stopPropagation();
		app.showMenu(event, branch ? repo.branchItems(branch) : repo.remoteBranchItems(label.name));
	}

	function menu(event: MouseEvent, commit: Commit) {
		const short = commit.hash.slice(0, 8);
		app.showMenu(event, [
			...(commit.hash === headHash ? [{ label: t('graph.undo'), action: () => repo.undoCommit() }] : []),
			{ label: t('graph.checkout'), action: () => repo.run(['checkout', commit.hash]), divider: commit.hash === headHash },
			{ label: t('branch.newHere'), action: () => repo.newBranch(commit.hash) },
			{ label: t('graph.newTag'), action: () => repo.newTag(commit.hash) },
			{ label: 'Cherry-pick', action: () => repo.run(['cherry-pick', commit.hash]) },
			{ label: t('graph.revert'), action: () => repo.run(['revert', '--no-edit', commit.hash]) },
			{ label: t('graph.resetSoft'), action: () => repo.reset(commit.hash, 'soft') },
			{ label: t('graph.resetMixed'), action: () => repo.reset(commit.hash, 'mixed') },
			{ label: t('graph.resetHard'), danger: true, action: () => repo.reset(commit.hash, 'hard') },
			{ label: t('graph.copyHash', { hash: short }), action: () => app.copy(commit.hash) }
		]);
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="scroller"
	onmouseleave={() => (hovered = null)}
	bind:this={scroller}
	bind:clientHeight={height}
	onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
>
	{#if !commits.length}
		<p class="empty muted">{t('graph.empty')}</p>
	{/if}
	<div class="canvas" style:height="{commits.length * ROW}px">
		<div class="window" style:transform="translateY({first * ROW}px)">
			{#each visible as commit, i (commit.hash)}
				{@const labels = parseRefs(commit.refs, repo.overview?.remotes ?? [])}
				{@const line = rows[first + i]?.line}
				{@const branch = lineNames.get(line)}
				<button
					class="commit"
					title={branch ? t('graph.branchOf', { name: branch.join(', ') }) : undefined}
					style:--lane={color(rows[first + i]?.color ?? 0)}
					onmouseenter={() => (hovered = line)}
					class:selected={repo.selected === commit.hash}
					onclick={() => (repo.selected = repo.selected === commit.hash ? null : commit.hash)}
					oncontextmenu={(e) => menu(e, commit)}
				>
					<span class="gap" style:width="{graphWidth}px"></span>
					<span class="subject">
						{#each labels as label (label.kind + label.name)}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<span
								class="ref mono"
								class:head={label.kind === 'head'}
								class:lit={hovered === line && label.kind !== 'tag'} oncontextmenu={(e) => refMenu(e, label)}>
								{#if label.kind === 'tag'}<Tag size={12} strokeWidth={1.5} />{/if}
								{#if label.kind === 'remote' || label.synced}<Cloud size={12} strokeWidth={1.5} />{/if}
								{label.name}
							</span>
						{/each}
						<span class="ellipsis text">{commit.subject}</span>
					</span>
					<span class="author ellipsis secondary">{commit.author}</span>
					<span class="date mono muted">{formatTime(commit.time)}</span>
					<span class="hash mono muted">{commit.hash.slice(0, 7)}</span>
				</button>
			{/each}

			<svg width={graphWidth} height={visible.length * ROW} aria-hidden="true">
				{#each rows.slice(Math.max(0, first - 1), last) as row, i (Math.max(0, first - 1) + i)}
					{#each row.segments as segment}
						<path
							d={path(segment, Math.max(0, first - 1) + i)}
							fill="none"
							stroke-width={hovered === segment.line ? 3 : 2}
							opacity={hovered === null || hovered === segment.line ? 1 : 0.35}
							style:stroke={color(segment.color)}
						/>
					{/each}
				{/each}
				{#each visible as commit, i (commit.hash)}
					{@const row = rows[first + i]}
					{@const x = laneX(row.lane)}
					{@const y = i * ROW + ROW / 2}
					{@const dim = hovered !== null && hovered !== row.line}
					{#if commit.hash === headHash}
						<path
							transform="translate({x - 8} {y - 8})"
							style:fill="var(--accent)"
							style:stroke="var(--bg)"
							stroke-width="1.5"
							d="M1 0 L5.4 3.4 Q8 2.8 10.6 3.4 L15 0 L14.6 8 Q14 13 8 16 Q2 13 1.4 8 Z"
						/>
					{:else if commit.parents.length > 1}
						<circle cx={x} cy={y} r="4" opacity={dim ? 0.35 : 1} style:fill="var(--bg)" style:stroke={color(row.color)} stroke-width="2" />
					{:else}
						<circle cx={x} cy={y} r="4.5" opacity={dim ? 0.35 : 1} style:fill={color(row.color)} style:stroke="var(--bg)" stroke-width="1.5" />
					{/if}
				{/each}
			</svg>
		</div>
	</div>
</div>

<style>
	.scroller {
		height: 100%;
		overflow-y: auto;
		overflow-x: hidden;
	}

	.canvas {
		position: relative;
		overflow: hidden;
	}

	.window {
		position: absolute;
		inset: 0 0 auto;
	}

	svg {
		position: absolute;
		top: 0;
		left: 0;
		overflow: hidden;
		pointer-events: none;
	}

	.commit {
		display: flex;
		align-items: center;
		gap: 16px;
		width: 100%;
		height: 32px;
		padding: 0 16px 0 0;
		border: 0;
		background: none;
		text-align: left;
		cursor: pointer;
		transition: background-color 120ms var(--ease);
	}

	.commit:hover {
		background: var(--hover);
	}

	.commit.selected {
		background: var(--raised);
	}

	.gap {
		flex: none;
	}

	.subject {
		display: flex;
		align-items: center;
		flex: 1;
		gap: 8px;
		min-width: 0;
	}

	.text {
		min-width: 0;
	}

	.ref {
		display: inline-flex;
		align-items: center;
		flex: none;
		gap: 4px;
		max-width: 200px;
		height: 20px;
		padding: 0 6px;
		border: 1px solid var(--line);
		border-radius: 4px;
		color: var(--text-2);
		white-space: nowrap;
		overflow: hidden;
	}

	.ref.head {
		border-color: color-mix(in srgb, var(--accent) 45%, transparent);
		color: var(--accent-ink);
	}

	.ref.lit {
		border-color: var(--lane);
		background: color-mix(in srgb, var(--lane) 22%, transparent);
		color: var(--text);
		transition: background-color 120ms var(--ease);
	}

	.author {
		flex: none;
		width: 136px;
	}

	.date {
		flex: none;
		width: 128px;
		text-align: right;
	}

	.hash {
		flex: none;
		width: 56px;
	}

	.empty {
		padding: 32px;
		text-align: center;
	}

	@media (max-width: 1180px) {
		.author {
			display: none;
		}
	}
</style>
