import { api } from './api';

export type Mode = 'system' | 'dark' | 'light';
type Shade = 'dark' | 'light';
type Ramp = Record<string, string>;

const ramps: Record<string, Record<Shade, Ramp>> = {
	graphite: {
		dark: {
			deep: '#0f0f10',
			bg: '#141415',
			card: '#19191b',
			raised: '#232325',
			text: '#ededed',
			'text-2': '#a1a1a6',
			'text-3': '#85858b'
		},
		light: {
			deep: '#f4f4f5',
			bg: '#fafafa',
			card: '#ffffff',
			raised: '#ececee',
			text: '#171717',
			'text-2': '#525252',
			'text-3': '#737373'
		}
	},
	warm: {
		dark: {
			deep: '#12110f',
			bg: '#161513',
			card: '#1c1a18',
			raised: '#262421',
			text: '#efe9de',
			'text-2': '#aaa398',
			'text-3': '#8a847a'
		},
		light: {
			deep: '#efe9df',
			bg: '#f7f4ee',
			card: '#fdfbf7',
			raised: '#e8e1d5',
			text: '#1f1b16',
			'text-2': '#5e554a',
			'text-3': '#7b7266'
		}
	},
	ink: {
		dark: {
			deep: '#07080d',
			bg: '#0b0d14',
			card: '#11141c',
			raised: '#1a1e28',
			text: '#eef0f5',
			'text-2': '#a3a9b8',
			'text-3': '#7a8296'
		},
		light: {
			deep: '#eceff4',
			bg: '#f5f7fa',
			card: '#ffffff',
			raised: '#e3e7ee',
			text: '#111827',
			'text-2': '#4b5565',
			'text-3': '#6b7487'
		}
	}
};

const lanes: Record<Shade, string[]> = {
	dark: ['#5b9dee', '#4cc38a', '#d9a441', '#e5689b', '#45b8c4', '#a78bda', '#9aa86a'],
	light: ['#2f6fc4', '#16874f', '#a8740f', '#c23a74', '#0f8391', '#7a5cc0', '#66752f']
};

export interface Palette {
	id: string;
	name: string;
	ramp: keyof typeof ramps;
	accent: string;
}

export const palettes: Palette[] = [
	{ id: 'fox', name: 'Лис', ramp: 'graphite', accent: '#f0803c' },
	{ id: 'taiga', name: 'Тайга', ramp: 'graphite', accent: '#6fae7c' },
	{ id: 'fjord', name: 'Фьорд', ramp: 'ink', accent: '#5b9dee' },
	{ id: 'cedar', name: 'Кедр', ramp: 'warm', accent: '#d9a441' }
];

export const SYSTEM_PALETTE = 'system';

const darkQuery = () => window.matchMedia('(prefers-color-scheme: dark)');

function luminance(hex: string) {
	const channel = (i: number) => {
		const c = parseInt(hex.slice(i, i + 2), 16) / 255;
		return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
	};
	return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

/** Picks the text colour with the higher contrast on the accent. */
function onAccent(accent: string) {
	const dark = '#141415';
	const l = luminance(accent);
	return 1.05 / (l + 0.05) > (l + 0.05) / (luminance(dark) + 0.05) ? '#ffffff' : dark;
}

function contrast(a: string, b: string) {
	const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
	return (hi + 0.05) / (lo + 0.05);
}

/** Moves a colour towards white or black until it stands out on the background. */
function legibleOn(background: string, color: string, shade: Shade) {
	const target = shade === 'dark' ? 255 : 0;
	const channels = [1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16));
	let result = color;
	for (let step = 0; step <= 10 && contrast(result, background) < 3; step++) {
		const mixed = channels.map((c) => Math.round(c + (target - c) * step * 0.1));
		result = '#' + mixed.map((c) => c.toString(16).padStart(2, '0')).join('');
	}
	return result;
}

export async function readSystemAccent() {
	try {
		return await api.systemAccent();
	} catch {
		return null;
	}
}

export function applyTheme(paletteId: string, mode: Mode, systemAccent: string | null) {
	const followSystem = paletteId === SYSTEM_PALETTE;
	const palette = palettes.find((p) => p.id === paletteId) ?? palettes[0];
	const effective = followSystem ? 'system' : mode;
	const shade: Shade =
		effective === 'system' ? (darkQuery().matches ? 'dark' : 'light') : effective;
	const ramp = ramps[followSystem ? 'graphite' : palette.ramp][shade];
	const accent = followSystem
		? legibleOn(ramp.bg, systemAccent ?? palettes[0].accent, shade)
		: palette.accent;

	const root = document.documentElement;
	for (const [name, value] of Object.entries(ramp)) root.style.setProperty(`--${name}`, value);
	root.style.setProperty('--accent', accent);
	root.style.setProperty('--on-accent', onAccent(accent));
	root.style.setProperty(
		'--accent-ink',
		shade === 'dark' ? accent : `color-mix(in oklab, ${accent} 72%, black)`
	);
	root.style.setProperty('--pos', shade === 'dark' ? '#4cc38a' : '#16a34a');
	root.style.setProperty('--neg', shade === 'dark' ? '#e5484d' : '#dc2626');
	root.style.setProperty('--warn', shade === 'dark' ? '#f0b44c' : '#b45309');
	root.style.setProperty('--lane-0', accent);
	lanes[shade].forEach((color, i) => root.style.setProperty(`--lane-${i + 1}`, color));
	root.style.colorScheme = shade;
}

export function onSystemThemeChange(listener: () => void) {
	darkQuery().addEventListener('change', listener);
}
