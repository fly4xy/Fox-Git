<script lang="ts">
	import { ChevronDown, ChevronRight } from '@lucide/svelte';
	import type { Snippet } from 'svelte';
	import type { FileChange } from '$lib/api';
	import { app, type Repo } from '$lib/state.svelte';
	import { treeRows, type FileRow } from '$lib/tree';

	let {
		repo,
		files,
		selected,
		onselect,
		onopen,
		onmenu,
		actions
	}: {
		repo: Repo;
		files: FileChange[];
		selected: Set<string>;
		onselect: (paths: Set<string>, focus?: FileChange) => void;
		onopen?: (file: FileChange) => void;
		onmenu?: (event: MouseEvent, files: FileChange[]) => void;
		actions?: Snippet<[FileChange]>;
	} = $props();

	let anchor: string | null = null;

	const tree = $derived(app.settings.fileView === 'tree');
	const rows: FileRow[] = $derived(
		tree
			? treeRows(files, repo.collapsed)
			: files.map((file) => ({ kind: 'file', file, name: file.path, depth: 0 }))
	);
	const order = $derived(rows.flatMap((row) => (row.kind === 'file' ? [row.file.path] : [])));

	function click(event: MouseEvent, file: FileChange) {
		const { path } = file;
		if (event.shiftKey && anchor) {
			const [from, to] = [order.indexOf(anchor), order.indexOf(path)].sort((a, b) => a - b);
			onselect(new Set(order.slice(from, to + 1)), file);
			return;
		}
		anchor = path;
		if (event.ctrlKey || event.metaKey) {
			const next = new Set(selected);
			if (!next.delete(path)) next.add(path);
			onselect(next, file);
		} else {
			onselect(new Set([path]), file);
		}
	}

	function clickFolder(event: MouseEvent, row: FileRow & { kind: 'dir' }) {
		if (event.ctrlKey || event.metaKey) {
			onselect(new Set([...selected, ...row.files.map((f) => f.path)]));
		} else if (!repo.collapsed.delete(row.path)) {
			repo.collapsed.add(row.path);
		}
	}

	function fileMenu(event: MouseEvent, file: FileChange) {
		if (selected.has(file.path)) {
			onmenu?.(event, files.filter((f) => selected.has(f.path)));
		} else {
			anchor = file.path;
			onselect(new Set([file.path]), file);
			onmenu?.(event, [file]);
		}
	}

	function folderMenu(event: MouseEvent, row: FileRow & { kind: 'dir' }) {
		onselect(new Set(row.files.map((f) => f.path)));
		onmenu?.(event, row.files);
	}
</script>

<ul>
	{#each rows as row (row.kind === 'dir' ? `dir:${row.path}` : row.file.path)}
		{#if row.kind === 'dir'}
			<li style:padding-left="{row.depth * 16}px">
				<button
					class="row folder"
					title={row.path}
					onclick={(e) => clickFolder(e, row)}
					oncontextmenu={(e) => folderMenu(e, row)}
				>
					{#if repo.collapsed.has(row.path)}
						<ChevronRight size={16} strokeWidth={1.5} />
					{:else}
						<ChevronDown size={16} strokeWidth={1.5} />
					{/if}
					<span class="ellipsis">{row.name}</span>
					<span class="mono muted count">{row.files.length}</span>
				</button>
			</li>
		{:else}
			<li class:active={selected.has(row.file.path)} style:padding-left="{row.depth * 16}px">
				<button
					class="row"
					title={row.file.path}
					onclick={(e) => click(e, row.file)}
					ondblclick={() => onopen?.(row.file)}
					oncontextmenu={(e) => fileMenu(e, row.file)}
				>
					<span class="code {row.file.code}">{row.file.code === '?' ? 'A' : row.file.code}</span>
					<span class="ellipsis">{row.name}</span>
				</button>
				{#if actions}{@render actions(row.file)}{/if}
			</li>
		{/if}
	{/each}
</ul>

<style>
	ul {
		flex: 1;
		min-height: 0;
		margin: 0;
		padding: 0 8px 8px;
		overflow-y: auto;
		list-style: none;
	}

	li {
		display: flex;
		align-items: center;
		gap: 2px;
		height: 28px;
		padding-right: 4px;
		border-radius: 6px;
		color: var(--text-2);
	}

	li:hover {
		background: var(--hover);
	}

	li.active {
		background: var(--raised);
		color: var(--text);
	}

	li :global(.icon-btn) {
		opacity: 0;
	}

	li:hover :global(.icon-btn),
	li :global(.icon-btn:focus-visible) {
		opacity: 1;
	}

	.row {
		display: flex;
		align-items: center;
		flex: 1;
		gap: 8px;
		min-width: 0;
		height: 100%;
		padding: 0 8px;
		border: 0;
		background: none;
		text-align: left;
		cursor: pointer;
	}

	.folder {
		padding-left: 8px;
		gap: 4px;
		color: var(--text-3);
	}

	.folder .ellipsis {
		color: var(--text-2);
	}

	.count {
		margin-left: auto;
	}
</style>
