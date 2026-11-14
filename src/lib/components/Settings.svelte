<script lang="ts">
	import { onMount } from 'svelte';
	import { api, type SshKey } from '$lib/api';
	import { setLanguage, t, type Language } from '$lib/i18n.svelte';
	import { app } from '$lib/state.svelte';
	import { palettes, SYSTEM_PALETTE, type Mode } from '$lib/theme';
	import Modal from './Modal.svelte';

	const languages: { id: Language; name: () => string }[] = [
		{ id: 'system', name: () => t('settings.langSystem') },
		{ id: 'en', name: () => 'English' },
		{ id: 'ru', name: () => 'Русский' }
	];
	const modes: { id: Mode; name: () => string }[] = [
		{ id: 'system', name: () => t('settings.modeSystem') },
		{ id: 'dark', name: () => t('settings.modeDark') },
		{ id: 'light', name: () => t('settings.modeLight') }
	];

	let builtIn = $state(true);
	let secret = $state('');
	let waiting = $state(false);
	let keys = $state<SshKey[]>([]);

	const followsSystem = $derived(app.settings.palette === SYSTEM_PALETTE);
	const canSignIn = $derived(builtIn || app.settings.clientId.trim().length > 0);

	const report = (error: unknown) => app.toast(String(error));
	const loadKeys = () => api.sshKeys().then((list) => (keys = list), report);

	onMount(() => {
		loadKeys();
		api.githubBuiltIn().then((value) => (builtIn = value));
		return () => {
			if (waiting) api.githubLoginCancel();
		};
	});

	function setLang(language: Language) {
		app.settings.lang = language;
		setLanguage(language);
		app.save();
	}

	function setPalette(id: string) {
		app.settings.palette = id;
		app.theme();
		app.save();
	}

	function setMode(mode: Mode) {
		app.settings.mode = mode;
		app.theme();
		app.save();
	}

	async function login() {
		app.save();
		waiting = true;
		try {
			await api.githubLogin(app.settings.clientId.trim(), secret.trim(), t('auth.done'));
			secret = '';
			app.user = await api.githubUser();
		} catch (error) {
			if (error !== 'cancelled') report(error);
		} finally {
			waiting = false;
		}
	}

	async function logout() {
		await api.githubLogout().catch(report);
		app.user = null;
	}

	async function generate() {
		const values = await app.ask({
			title: t('ssh.newTitle'),
			message: t('ssh.newMessage'),
			fields: [
				{ key: 'name', label: t('ssh.fileName'), value: 'id_ed25519' },
				{ key: 'comment', label: t('ssh.comment'), value: 'fox-git' }
			],
			confirm: t('common.create')
		});
		if (!values) return;
		await app
			.track(t('busy.sshGenerate'), true, () => api.sshGenerate(values.name, values.comment))
			.catch(report);
		await loadKeys();
	}

	async function upload(key: SshKey) {
		try {
			await api.githubAddSshKey(`Fox Git ${key.name}`, key.public_key);
			app.toast(t('ssh.added', { name: key.name }), 'info');
		} catch (error) {
			report(error);
		}
	}

	function testSsh() {
		app.track(t('busy.sshTest'), false, () => api.sshTest()).then((message) => app.toast(message, 'info'), report);
	}
</script>

<Modal title={t('settings.title')} width={560} onclose={() => (app.settingsOpen = false)}>
	<div class="settings">
		<section>
			<h3>{t('settings.language')}</h3>
			<div class="modes">
				{#each languages as language (language.id)}
					<button class="btn" class:ghost={app.settings.lang !== language.id} onclick={() => setLang(language.id)}>
						{language.name()}
					</button>
				{/each}
			</div>
		</section>

		<section>
			<h3>{t('settings.window')}</h3>
			<label class="check">
				<input
					type="checkbox"
					checked={app.settings.systemTitleBar}
					onchange={(e) => {
						app.settings.systemTitleBar = e.currentTarget.checked;
						app.applyTitleBar();
						app.save();
					}}
				/>
				{t('settings.systemTitleBar')}
			</label>
		</section>

		<section>
			<h3>{t('settings.palette')}</h3>
			<div class="palettes">
				{#each palettes as palette (palette.id)}
					<button class="palette" class:active={app.settings.palette === palette.id} onclick={() => setPalette(palette.id)}>
						<span class="swatch" style:background={palette.accent}></span>
						{t(`palette.${palette.id}` as 'palette.fox')}
					</button>
				{/each}
				<button class="palette" class:active={followsSystem} onclick={() => setPalette(SYSTEM_PALETTE)}>
					<span class="swatch" style:background={app.systemAccent ?? palettes[0].accent}></span>
					{t('settings.paletteSystem')}
				</button>
			</div>
			{#if followsSystem}
				<p class="muted hint">{app.systemAccent ? t('settings.systemHint') : t('settings.systemNoAccent')}</p>
			{:else}
				<div class="modes spaced">
					{#each modes as mode (mode.id)}
						<button class="btn" class:ghost={app.settings.mode !== mode.id} onclick={() => setMode(mode.id)}>
							{mode.name()}
						</button>
					{/each}
				</div>
			{/if}
		</section>

		<section>
			<h3>GitHub</h3>
			{#if app.user}
				<div class="account">
					<img src={app.user.avatar_url} alt="" width="32" height="32" />
					<span class="grow">{app.user.name ?? app.user.login}<span class="muted"> {app.user.login}</span></span>
					<button class="btn ghost" onclick={logout}>{t('auth.signOut')}</button>
				</div>
			{:else if waiting}
				<div class="account">
					<span class="grow secondary">{t('auth.waiting')}</span>
					<button class="btn ghost" onclick={() => api.githubLoginCancel()}>{t('common.cancel')}</button>
				</div>
			{:else}
				{#if !builtIn}
					<label class="field">
						<span class="secondary">{t('auth.clientId')}</span>
						<input class="input mono" bind:value={app.settings.clientId} />
					</label>
					<label class="field">
						<span class="secondary">{t('auth.clientSecret')}</span>
						<input class="input mono" type="password" bind:value={secret} />
					</label>
				{/if}
				<button class="btn primary" disabled={!canSignIn} onclick={login}>{t('auth.signIn')}</button>
			{/if}
		</section>

		<section>
			<div class="title">
				<h3>{t('ssh.title')}</h3>
				<button class="btn ghost" onclick={testSsh}>{t('ssh.test')}</button>
				<button class="btn ghost" onclick={generate}>{t('ssh.create')}</button>
			</div>
			{#each keys as key (key.name)}
				<div class="key">
					<span class="mono grow ellipsis" title={key.public_key}>{key.name}</span>
					<button class="btn ghost" onclick={() => app.copy(key.public_key)}>{t('common.copy')}</button>
					{#if app.user}<button class="btn ghost" onclick={() => upload(key)}>{t('ssh.toGithub')}</button>{/if}
				</div>
			{:else}
				<p class="muted">{t('ssh.none')}</p>
			{/each}
		</section>
	</div>
</Modal>

<style>
	.settings {
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	h3 {
		margin-bottom: 8px;
		font-size: 13px;
	}

	.palettes {
		display: grid;
		grid-template-columns: repeat(5, 1fr);
		gap: 8px;
	}

	.palette {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 8px;
		padding: 8px;
		border: 1px solid var(--line);
		border-radius: 10px;
		background: none;
		color: var(--text-2);
		font-size: 12px;
		line-height: 16px;
		text-align: left;
		cursor: pointer;
		transition: border-color 140ms var(--ease);
	}

	.palette:hover {
		border-color: var(--line-strong);
	}

	.palette.active {
		border-color: var(--accent);
		color: var(--text);
	}

	.swatch {
		width: 100%;
		height: 24px;
		border-radius: 4px;
	}

	.hint {
		margin-top: 8px;
	}

	.modes {
		display: flex;
		gap: 4px;
	}

	.spaced {
		margin-top: 8px;
	}

	.account,
	.key,
	.title {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.title {
		margin-bottom: 4px;
	}

	.title h3 {
		flex: 1;
		margin: 0;
	}

	.key {
		height: 40px;
		border-top: 1px solid var(--line);
	}

	img {
		border-radius: 8px;
	}

	.grow {
		flex: 1;
		min-width: 0;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 4px;
		margin-bottom: 8px;
	}
</style>
