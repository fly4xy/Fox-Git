<script lang="ts">
	import { FolderTree, List } from '@lucide/svelte';
	import { t } from '$lib/i18n.svelte';
	import { app } from '$lib/state.svelte';

	function set(view: 'list' | 'tree') {
		app.settings.fileView = view;
		app.save();
	}
</script>

<div class="toggle">
	<button
		class="icon-btn"
		class:on={app.settings.fileView === 'list'}
		title={t('files.list')}
		aria-label={t('files.list')}
		onclick={() => set('list')}
	>
		<List size={16} strokeWidth={1.5} />
	</button>
	<button
		class="icon-btn"
		class:on={app.settings.fileView === 'tree'}
		title={t('files.tree')}
		aria-label={t('files.tree')}
		onclick={() => set('tree')}
	>
		<FolderTree size={16} strokeWidth={1.5} />
	</button>
</div>

<style>
	.toggle {
		display: flex;
		flex: none;
		gap: 2px;
	}

	.on {
		background: var(--raised);
		color: var(--text);
	}
</style>
