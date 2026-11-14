export interface DiffLine {
	kind: 'add' | 'del' | 'ctx' | 'note';
	oldNo: number | null;
	newNo: number | null;
	text: string;
}

export interface Hunk {
	title: string;
	raw: string;
	lines: DiffLine[];
}

export interface ParsedDiff {
	header: string;
	hunks: Hunk[];
	binary: boolean;
}

const HUNK_START = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@(.*)$/;

export function parseDiff(patch: string): ParsedDiff {
	const lines = patch.split('\n');
	if (lines[lines.length - 1] === '') lines.pop();

	const headerEnd = lines.findIndex((line) => line.startsWith('@@ '));
	const headerLines = headerEnd < 0 ? lines : lines.slice(0, headerEnd);
	const diff: ParsedDiff = {
		header: headerLines.join('\n') + '\n',
		hunks: [],
		binary: headerLines.some((l) => l.startsWith('Binary files') || l === 'GIT binary patch')
	};
	if (headerEnd < 0) return diff;

	let hunk: Hunk | null = null;
	let oldNo = 0;
	let newNo = 0;
	for (const line of lines.slice(headerEnd)) {
		const start = HUNK_START.exec(line);
		if (start) {
			oldNo = Number(start[1]);
			newNo = Number(start[2]);
			hunk = { title: start[3].trim(), raw: '', lines: [] };
			diff.hunks.push(hunk);
		} else if (hunk) {
			const text = line.slice(1);
			if (line[0] === '+') hunk.lines.push({ kind: 'add', oldNo: null, newNo: newNo++, text });
			else if (line[0] === '-') hunk.lines.push({ kind: 'del', oldNo: oldNo++, newNo: null, text });
			else if (line[0] === '\\') hunk.lines.push({ kind: 'note', oldNo: null, newNo: null, text });
			else hunk.lines.push({ kind: 'ctx', oldNo: oldNo++, newNo: newNo++, text });
		}
		if (hunk) hunk.raw += line + '\n';
	}
	return diff;
}
