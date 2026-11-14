export type LogLevel = 'info' | 'warning' | 'error';

export interface LogEntry {
	id: number;
	time: number;
	level: LogLevel;
	text: string;
}

const LIMIT = 500;
let lastId = 0;

/** What git ran and what went wrong, newest last. Errors open the panel. */
export const log = $state({ entries: [] as LogEntry[], open: false });

export function record(level: LogLevel, text: string) {
	log.entries.push({ id: ++lastId, time: Date.now(), level, text });
	if (log.entries.length > LIMIT) log.entries.splice(0, log.entries.length - LIMIT);
	if (level === 'error') log.open = true;
}
