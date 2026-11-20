<script lang="ts">
	import { getVersion } from '@tauri-apps/api/app';
	import { t } from '$lib/i18n.svelte';
	import { app } from '$lib/state.svelte';
	import Modal from './Modal.svelte';

	const update = $derived(app.update!);
	const busy = $derived(update.progress !== null);

	let current = $state('');
	getVersion().then((v) => (current = v), () => {});

	const close = () => {
		if (!busy) app.update = null;
	};
</script>

<Modal
	title={t(update.installed ? 'update.installed' : 'update.title', { version: update.version })}
	onclose={close}
>
	<div class="update">
		{#if update.installed}
			<p>{t('update.restartHint')}</p>
		{:else}
			<p>{t('update.message', { current: current || '…' })}</p>
			{#if !update.installable}<p class="muted">{t('update.manual')}</p>{/if}
		{/if}
		{#if busy}
			<div class="bar" role="progressbar" aria-valuenow={update.progress}><span style:width="{update.progress}%"></span></div>
		{/if}
		<footer>
			<button class="btn ghost" disabled={busy} onclick={close}>{t('update.later')}</button>
			{#if update.installed}
				<button class="btn primary" onclick={() => app.restartForUpdate()}>{t('update.restart')}</button>
			{:else}
				<button class="btn primary" disabled={busy} onclick={() => app.installUpdate()}>
					{#if busy}
						{t('update.downloading', { percent: update.progress ?? 0 })}
					{:else}
						{update.installable ? t('update.install') : t('update.download')}
					{/if}
				</button>
			{/if}
		</footer>
	</div>
</Modal>

<style>
	.update {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.bar {
		height: 4px;
		border-radius: 2px;
		background: var(--raised);
		overflow: hidden;
	}

	.bar span {
		display: block;
		height: 100%;
		background: var(--accent);
		transition: width 160ms var(--ease);
	}

	footer {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 4px;
	}
</style>
