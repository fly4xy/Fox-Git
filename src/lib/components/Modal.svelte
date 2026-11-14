<script lang="ts">
	import { X } from '@lucide/svelte';
	import type { Snippet } from 'svelte';
	import { t } from '$lib/i18n.svelte';

	let {
		title,
		width = 440,
		onclose,
		children
	}: { title: string; width?: number; onclose: () => void; children: Snippet } = $props();
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div class="backdrop" onpointerdown={(e) => e.target === e.currentTarget && onclose()}>
	<div class="modal" role="dialog" aria-label={title} style:width="{width}px">
		<header>
			<h2>{title}</h2>
			<button class="icon-btn" aria-label={t('common.close')} onclick={onclose}>
				<X size={16} strokeWidth={1.5} />
			</button>
		</header>
		<div class="content">{@render children()}</div>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		z-index: 20;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 24px;
		background: rgb(0 0 0 / 0.45);
		animation: fade 160ms var(--ease);
	}

	.modal {
		display: flex;
		flex-direction: column;
		max-width: 100%;
		max-height: 100%;
		border: 1px solid var(--line-strong);
		border-radius: 14px;
		background: var(--card);
		animation: rise 220ms var(--ease);
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		flex: none;
		padding: 16px 16px 12px 20px;
	}

	h2 {
		font-size: 15px;
		line-height: 20px;
	}

	.content {
		min-height: 0;
		padding: 0 20px 20px;
		overflow-y: auto;
	}

	@keyframes fade {
		from {
			opacity: 0;
		}
	}

	@keyframes rise {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
	}
</style>
