<script lang="ts">
	import { Download, Plus, X } from '@lucide/svelte';
	import { t } from '$lib/i18n.svelte';
	import { app, type Repo } from '$lib/state.svelte';

	let { repo }: { repo: Repo } = $props();

	const lfs = $derived(repo.lfs);

	async function track() {
		const values = await app.ask({
			title: t('lfs.trackTitle'),
			fields: [{ key: 'pattern', label: t('lfs.pattern') }],
			confirm: t('common.add')
		});
		if (values?.pattern) await repo.run(['lfs', 'track', values.pattern], t('busy.lfs'));
	}
</script>

<div class="lfs">
	{#if lfs && !lfs.installed}
		<p class="muted empty">{t('lfs.missing')}</p>
	{:else if lfs}
		<header>
			<h2>Git LFS</h2>
			<button class="btn ghost" onclick={() => repo.run(['lfs', 'install', '--local'], t('busy.lfs'))}>
				{t('lfs.enable')}
			</button>
			<button class="btn" onclick={() => repo.run(['lfs', 'pull'], t('busy.lfsPull'))}>
				<Download size={16} strokeWidth={1.5} />{t('lfs.download')}
			</button>
		</header>

		<section>
			<div class="title">
				<h3>{t('lfs.patterns')}</h3>
				<button class="btn ghost" onclick={track}><Plus size={16} strokeWidth={1.5} />{t('common.add')}</button>
			</div>
			{#each lfs.patterns as pattern (pattern)}
				<div class="row">
					<span class="mono grow">{pattern}</span>
					<button
						class="icon-btn"
						aria-label={t('lfs.untrack')}
						onclick={() => repo.run(['lfs', 'untrack', pattern], t('busy.lfs'))}
					>
						<X size={16} strokeWidth={1.5} />
					</button>
				</div>
			{:else}
				<p class="muted">{t('lfs.noPatterns')}</p>
			{/each}
		</section>

		<section>
			<div class="title">
				<h3>{t('lfs.files')}</h3>
				<span class="mono muted">{lfs.files.length}</span>
			</div>
			{#each lfs.files as file (file.path)}
				<div class="row">
					<span class="grow ellipsis" title={file.path}>{file.path}</span>
					<span class="mono muted">{file.size}</span>
					<span class="state muted">{file.local ? t('lfs.local') : t('lfs.pointer')}</span>
				</div>
			{:else}
				<p class="muted">{t('lfs.noFiles')}</p>
			{/each}
		</section>
	{/if}
</div>

<style>
	.lfs {
		display: flex;
		flex-direction: column;
		gap: 24px;
		height: 100%;
		max-width: 960px;
		padding: 24px;
		overflow-y: auto;
	}

	header {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	h2 {
		flex: 1;
		font-size: 18px;
		line-height: 24px;
	}

	.title {
		display: flex;
		align-items: center;
		justify-content: space-between;
		min-height: 32px;
		margin-bottom: 4px;
	}

	h3 {
		font-size: 14px;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 16px;
		height: 32px;
		border-top: 1px solid var(--line);
	}

	.grow {
		flex: 1;
		min-width: 0;
	}

	.state {
		flex: none;
		width: 72px;
		text-align: right;
	}

	.empty {
		padding: 32px;
		text-align: center;
	}
</style>
