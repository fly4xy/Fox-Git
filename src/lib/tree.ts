import type { FileChange } from './api';

interface Folder {
	folders: Map<string, Folder>;
	files: { name: string; file: FileChange }[];
}

export type FileRow =
	| { kind: 'dir'; path: string; name: string; depth: number; files: FileChange[] }
	| { kind: 'file'; file: FileChange; name: string; depth: number };

function collect(folder: Folder): FileChange[] {
	return [...folder.files.map((f) => f.file), ...[...folder.folders.values()].flatMap(collect)];
}

function walk(folder: Folder, prefix: string, depth: number, collapsed: Set<string>, rows: FileRow[]) {
	const byName = (a: [string, Folder], b: [string, Folder]) => a[0].localeCompare(b[0]);
	for (const [segment, child] of [...folder.folders].sort(byName)) {
		// A chain of single folders reads better as one row: src/lib/components.
		let name = segment;
		let node = child;
		while (node.files.length === 0 && node.folders.size === 1) {
			const [next, inner] = [...node.folders][0];
			name += `/${next}`;
			node = inner;
		}
		const path = prefix + name;
		rows.push({ kind: 'dir', path, name, depth, files: collect(node) });
		if (!collapsed.has(path)) walk(node, `${path}/`, depth + 1, collapsed, rows);
	}
	for (const { name, file } of [...folder.files].sort((a, b) => a.name.localeCompare(b.name))) {
		rows.push({ kind: 'file', file, name, depth });
	}
}

/** Flattens files into the rows of a folder tree, skipping collapsed folders. */
export function treeRows(files: FileChange[], collapsed: Set<string>): FileRow[] {
	const root: Folder = { folders: new Map(), files: [] };
	for (const file of files) {
		const parts = file.path.split('/');
		const name = parts.pop()!;
		let folder = root;
		for (const part of parts) {
			if (!folder.folders.has(part)) folder.folders.set(part, { folders: new Map(), files: [] });
			folder = folder.folders.get(part)!;
		}
		folder.files.push({ name, file });
	}
	const rows: FileRow[] = [];
	walk(root, '', 0, collapsed, rows);
	return rows;
}
