<script module lang="ts">
	let lastId = 0;
</script>

<script lang="ts">
	import { Channel } from '@tauri-apps/api/core';
	import { FitAddon } from '@xterm/addon-fit';
	import { Terminal } from '@xterm/xterm';
	import '@xterm/xterm/css/xterm.css';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { app, type Repo } from '$lib/state.svelte';

	const REFRESH_DELAY = 600;

	let { repo, visible }: { repo: Repo; visible: boolean } = $props();

	let host: HTMLDivElement;
	let term = $state.raw<Terminal | null>(null);

	function palette() {
		const style = getComputedStyle(document.documentElement);
		const token = (name: string) => style.getPropertyValue(name).trim();
		return {
			background: token('--deep'),
			foreground: token('--text'),
			cursor: token('--text'),
			selectionBackground: 'rgba(128, 128, 128, 0.35)'
		};
	}

	onMount(() => {
		const id = ++lastId;
		const terminal = new Terminal({
			cursorBlink: true,
			fontFamily: "'Geist Mono Variable', ui-monospace, monospace",
			fontSize: 13,
			scrollback: 5000,
			theme: palette()
		});
		const fit = new FitAddon();
		terminal.loadAddon(fit);
		terminal.open(host);
		fit.fit();
		term = terminal;

		terminal.attachCustomKeyEventHandler((event) => {
			if (event.type !== 'keydown' || !event.ctrlKey) return true;
			const key = event.key.toLowerCase();
			if (key === 'c' && (event.shiftKey || terminal.hasSelection())) {
				app.copy(terminal.getSelection());
				terminal.clearSelection();
				return false;
			}
			// The webview pastes on its own, so the key only has to stay out of the shell.
			return key !== 'v';
		});

		// A command typed here can change the repository, so the app follows once output settles.
		let commandRan = false;
		let refresh: ReturnType<typeof setTimeout>;
		const output = new Channel<number[]>();
		output.onmessage = (data) => {
			if (!data.length) {
				if (app.active === repo) app.terminalOpen = false;
				repo.terminal = false;
				return;
			}
			terminal.write(new Uint8Array(data));
			if (!commandRan) return;
			clearTimeout(refresh);
			refresh = setTimeout(() => {
				commandRan = false;
				repo.refresh().catch(() => {});
			}, REFRESH_DELAY);
		};
		terminal.onData((data) => {
			if (data.includes('\r')) commandRan = true;
			api.terminalWrite(id, data);
		});
		terminal.onResize(({ cols, rows }) => api.terminalResize(id, cols, rows));
		api
			.terminalOpen(id, repo.path, terminal.cols, terminal.rows, output)
			.catch((error) => terminal.write(String(error)));

		const observer = new ResizeObserver(() => {
			if (host.clientWidth && host.clientHeight) fit.fit();
		});
		observer.observe(host);

		return () => {
			clearTimeout(refresh);
			observer.disconnect();
			api.terminalClose(id);
			terminal.dispose();
		};
	});

	$effect(() => {
		if (visible) term?.focus();
	});

	$effect(() => {
		app.themeRevision;
		if (term) term.options.theme = palette();
	});
</script>

<div class="pane" class:visible>
	<div class="host" bind:this={host}></div>
</div>

<style>
	.pane {
		display: none;
		height: 100%;
		padding: 8px 4px 8px 12px;
	}

	.pane.visible {
		display: block;
	}

	.host {
		height: 100%;
		user-select: text;
	}
</style>
