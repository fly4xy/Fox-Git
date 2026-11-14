<script lang="ts">
	import { Download, FolderOpen } from '@lucide/svelte';
	import { t } from '$lib/i18n.svelte';
	import { app } from '$lib/state.svelte';
	import FoxMark from './FoxMark.svelte';

	const name = (path: string) => path.split(/[\\/]/).filter(Boolean).pop() ?? path;
</script>

<div class="welcome">
	<section class="start">
		<div class="logo"><FoxMark size={28} /><h1>Fox Git</h1></div>
		<p class="secondary">{t('welcome.subtitle')}</p>
		<p class="muted">{t('welcome.drop')}</p>
		<div class="actions">
			<button class="btn primary" onclick={() => app.openDialog()}>
				<FolderOpen size={16} strokeWidth={1.5} />{t('welcome.open')}
			</button>
			<button class="btn" onclick={() => (app.cloneOpen = true)}>
				<Download size={16} strokeWidth={1.5} />{t('welcome.clone')}
			</button>
		</div>

		{#if app.settings.recent.length}
			<h2 class="muted">{t('welcome.recent')}</h2>
			<ul>
				{#each app.settings.recent as path (path)}
					<li>
						<button onclick={() => app.openPath(path)} title={path}>
							<span>{name(path)}</span>
							<span class="mono muted ellipsis">{path}</span>
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</section>

	<svg viewBox="0 0 420 560" aria-hidden="true">
		<g fill="none" stroke-width="3" stroke-linecap="round">
			<path style:stroke="var(--lane-1)" d="M140 0V120C140 170 230 170 230 220V340C230 390 140 390 140 440" />
			<path style:stroke="var(--lane-2)" d="M140 180C140 230 320 230 320 280V400C320 450 140 450 140 500" />
			<path style:stroke="var(--lane-0)" d="M140 0V560" />
		</g>
		{#each [[140, 60, 1], [140, 120, 1], [230, 260, 1], [230, 320, 1], [320, 300, 2], [320, 380, 2]] as [x, y, lane]}
			<circle cx={x} cy={y} r="8" style:fill="var(--lane-{lane})" style:stroke="var(--bg)" stroke-width="3" />
		{/each}
		{#each [240, 340, 440, 500] as y}
			<circle cx="140" cy={y} r="8" style:fill="var(--lane-0)" style:stroke="var(--bg)" stroke-width="3" />
		{/each}
	</svg>
</div>

<style>
	.welcome {
		display: flex;
		align-items: center;
		justify-content: center;
		flex: 1;
		gap: 64px;
		min-height: 0;
		padding: 32px;
		overflow: hidden;
	}

	.start {
		display: flex;
		flex-direction: column;
		width: 440px;
		max-height: 100%;
		min-height: 0;
	}

	.logo {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 8px;
	}

	h1 {
		font-size: 28px;
		line-height: 32px;
	}

	.actions {
		display: flex;
		gap: 8px;
		margin-top: 24px;
	}

	.actions .btn {
		height: 36px;
	}

	h2 {
		margin: 32px 0 8px;
		font-size: 13px;
		font-weight: 400;
	}

	ul {
		margin: 0 -8px;
		padding: 0;
		overflow-y: auto;
		list-style: none;
	}

	li button {
		display: flex;
		flex-direction: column;
		gap: 2px;
		width: 100%;
		padding: 8px;
		border: 0;
		border-radius: 8px;
		background: none;
		text-align: left;
		cursor: pointer;
		transition: background-color 140ms var(--ease);
	}

	li button:hover {
		background: var(--hover);
	}

	.ellipsis {
		max-width: 100%;
	}

	svg {
		flex: none;
		width: 340px;
		height: min(560px, 100%);
	}

	@media (max-width: 1040px) {
		svg {
			display: none;
		}
	}
</style>
