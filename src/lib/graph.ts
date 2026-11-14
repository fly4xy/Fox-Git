import type { Commit } from './api';

export const LANE_COLORS = 8;

/** A line between the centre of a row and the centre of the next one. */
export interface Segment {
	from: number;
	to: number;
	color: number;
}

export interface GraphRow {
	lane: number;
	color: number;
	segments: Segment[];
}

export interface GraphLayout {
	rows: GraphRow[];
	width: number;
}

/** Assigns commits, listed children first, to lanes. */
export function layoutGraph(commits: Commit[]): GraphLayout {
	const lanes: (string | null)[] = [];
	const colors: number[] = [];
	const rows: GraphRow[] = [];
	let nextColor = 0;
	let width = 1;

	const claimLane = () => {
		let lane = lanes.indexOf(null);
		if (lane < 0) lane = lanes.push(null) - 1;
		colors[lane] = nextColor++ % LANE_COLORS;
		return lane;
	};

	commits.forEach((commit, index) => {
		const incoming = lanes.flatMap((hash, lane) => (hash === commit.hash ? [lane] : []));
		const lane = incoming.length ? incoming[0] : claimLane();
		const color = colors[lane];

		// Extra children of this commit bend into its lane in the gap above.
		for (const other of incoming.slice(1)) {
			lanes[other] = null;
			for (const segment of rows[index - 1]?.segments ?? []) {
				if (segment.to === other) segment.to = lane;
			}
		}
		lanes[lane] = null;

		const segments: Segment[] = [];
		lanes.forEach((hash, l) => {
			if (hash) segments.push({ from: l, to: l, color: colors[l] });
		});

		commit.parents.forEach((parent, order) => {
			let target = lanes.indexOf(parent);
			if (target < 0) {
				target = order === 0 ? lane : claimLane();
				lanes[target] = parent;
			}
			segments.push({ from: lane, to: target, color: colors[target] });
		});

		rows.push({ lane, color, segments });
		width = Math.max(width, lanes.length);
	});

	return { rows, width };
}

export interface RefLabel {
	name: string;
	kind: 'head' | 'branch' | 'remote' | 'tag';
	synced: boolean;
}

/** Turns `git log %D` output into labels; a local branch absorbs its remote twin. */
export function parseRefs(raw: string, remoteNames: string[]): RefLabel[] {
	if (!raw) return [];
	const labels: RefLabel[] = [];
	const remotes: string[] = [];
	for (const part of raw.split(', ')) {
		if (part === 'HEAD') continue;
		if (part.startsWith('HEAD -> ')) {
			labels.push({ name: part.slice(8), kind: 'head', synced: false });
		} else if (part.startsWith('tag: ')) {
			labels.push({ name: part.slice(5), kind: 'tag', synced: false });
		} else if (remoteNames.some((name) => part.startsWith(`${name}/`))) {
			remotes.push(part);
		} else {
			labels.push({ name: part, kind: 'branch', synced: false });
		}
	}
	for (const remote of remotes) {
		if (remote.endsWith('/HEAD')) continue;
		const short = remote.slice(remote.indexOf('/') + 1);
		const local = labels.find((l) => l.name === short && l.kind !== 'tag');
		if (local) local.synced = true;
		else labels.push({ name: remote, kind: 'remote', synced: false });
	}
	return labels;
}
