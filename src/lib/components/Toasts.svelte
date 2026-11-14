<script lang="ts">
	import { X } from '@lucide/svelte';
	import { t } from '$lib/i18n.svelte';
	import { app } from '$lib/state.svelte';
</script>

<div class="toasts">
	{#each app.toasts as toast (toast.id)}
		<div class="toast" role="alert">
			<p class="selectable">{toast.text}</p>
			<button class="icon-btn" aria-label={t('common.close')} onclick={() => app.dismissToast(toast.id)}>
				<X size={16} strokeWidth={1.5} />
			</button>
		</div>
	{/each}
</div>

<style>
	.toasts {
		position: fixed;
		right: 16px;
		bottom: 16px;
		z-index: 40;
		display: flex;
		flex-direction: column;
		gap: 8px;
		width: 400px;
		max-width: calc(100vw - 32px);
	}

	.toast {
		display: flex;
		align-items: flex-start;
		gap: 8px;
		padding: 12px 8px 12px 16px;
		border: 1px solid var(--line-strong);
		border-radius: 12px;
		background: var(--raised);
		box-shadow: 0 8px 24px rgb(0 0 0 / 0.24);
		animation: rise 220ms var(--ease);
	}

	p {
		flex: 1;
		min-width: 0;
		max-height: 200px;
		overflow: auto;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	@keyframes rise {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
	}
</style>
