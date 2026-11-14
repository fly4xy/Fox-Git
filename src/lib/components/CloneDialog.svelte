<script lang="ts">
	import { open } from '@tauri-apps/plugin-dialog';
	import { onMount } from 'svelte';
	import { api, type GithubRepo } from '$lib/api';
	import { t } from '$lib/i18n.svelte';
	import { app } from '$lib/state.svelte';
	import Modal from './Modal.svelte';

	let url = $state('');
	let parent = $state('');
	let search = $state('');
	let useSsh = $state(false);
	let repos = $state<GithubRepo[]>([]);

	const separator = $derived(parent.includes('\\') ? '\\' : '/');
	const folder = $derived(url.split('/').pop()?.replace(/\.git$/, '').split(':').pop() ?? '');
	const dest = $derived(parent && folder ? `${parent}${separator}${folder}` : '');
	const found = $derived(
		repos.filter((r) => r.full_name.toLowerCase().includes(search.toLowerCase())).slice(0, 50)
	);

	onMount(() => {
		if (!app.user) return;
		api.githubRepos().then(
			(list) => (repos = list),
			(error) => app.toast(String(error))
		);
	});

	async function choose() {
		const path = await open({ directory: true, title: t('clone.where') });
		if (typeof path === 'string') parent = path;
	}
</script>

<Modal title={t('clone.title')} width={520} onclose={() => (app.cloneOpen = false)}>
	<form
		onsubmit={(e) => {
			e.preventDefault();
			app.clone(url.trim(), dest);
		}}
	>
		{#if app.user}
			<input class="input" placeholder={t('clone.search')} bind:value={search} />
			<ul>
				{#each found as repo (repo.full_name)}
					<li>
						<button type="button" onclick={() => (url = useSsh ? repo.ssh_url : repo.clone_url)}>
							<span class="ellipsis">{repo.full_name}</span>
							{#if repo.private}<span class="muted">{t('clone.private')}</span>{/if}
						</button>
					</li>
				{/each}
			</ul>
			<label class="check secondary"><input type="checkbox" bind:checked={useSsh} />{t('clone.ssh')}</label>
		{/if}
		<label class="field">
			<span class="secondary">{t('clone.url')}</span>
			<input class="input mono" placeholder="git@github.com:owner/repo.git" bind:value={url} />
		</label>
		<div class="field">
			<span class="secondary">{t('clone.folder')}</span>
			<div class="pick">
				<input class="input" readonly value={dest || parent} />
				<button class="btn" type="button" onclick={choose}>{t('clone.pick')}</button>
			</div>
		</div>
		<footer>
			<button class="btn primary" type="submit" disabled={!url.trim() || !dest || app.working}>
				{app.working ? app.task?.label : t('clone.confirm')}
			</button>
		</footer>
	</form>
</Modal>

<style>
	form {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	ul {
		max-height: 200px;
		margin: 0;
		padding: 0;
		overflow-y: auto;
		list-style: none;
	}

	li button {
		display: flex;
		justify-content: space-between;
		gap: 16px;
		width: 100%;
		height: 32px;
		align-items: center;
		padding: 0 8px;
		border: 0;
		border-radius: 8px;
		background: none;
		text-align: left;
		cursor: pointer;
	}

	li button:hover {
		background: var(--hover);
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.pick {
		display: flex;
		gap: 8px;
	}

	footer {
		display: flex;
		justify-content: flex-end;
	}
</style>
