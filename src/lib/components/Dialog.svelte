<script lang="ts">
	import { t } from '$lib/i18n.svelte';
	import { app, type DialogField, type DialogRequest } from '$lib/state.svelte';
	import Modal from './Modal.svelte';

	let { request }: { request: DialogRequest } = $props();

	// svelte-ignore state_referenced_locally
	const values = $state<Record<string, any>>(
		Object.fromEntries((request.fields ?? []).map((f) => [f.key, f.value ?? (f.type === 'checkbox' ? false : '')]))
	);

	const optionsOf = (field: DialogField) =>
		typeof field.options === 'function' ? field.options(values) : (field.options ?? []);

	$effect(() => {
		for (const field of request.fields ?? []) {
			if (field.type !== 'select') continue;
			const options = optionsOf(field);
			if (!options.includes(values[field.key])) values[field.key] = options[0] ?? '';
		}
	});

	function finish(result: Record<string, any> | null) {
		const { resolve } = request;
		app.dialog = null;
		resolve(result);
	}

	const focus = (node: HTMLInputElement) => node.focus();
</script>

<Modal title={request.title} onclose={() => finish(null)}>
	<form
		onsubmit={(e) => {
			e.preventDefault();
			finish(values);
		}}
	>
		{#if request.message}<p class="secondary selectable">{request.message}</p>{/if}
		{#each request.fields ?? [] as field, i (field.key)}
			{#if field.type === 'checkbox'}
				<label class="check secondary">
					<input type="checkbox" bind:checked={values[field.key]} />
					<span class="ellipsis">{field.label}</span>
				</label>
			{:else if field.type === 'select'}
				<label class="field">
					<span class="secondary">{field.label}</span>
					<select class="input" bind:value={values[field.key]}>
						{#each optionsOf(field) as option (option)}
							<option value={option}>{option}</option>
						{/each}
					</select>
				</label>
			{:else}
				<label class="field">
					<span class="secondary">{field.label}</span>
					{#if i === 0}
						<input class="input" bind:value={values[field.key]} use:focus />
					{:else}
						<input class="input" bind:value={values[field.key]} />
					{/if}
				</label>
			{/if}
		{/each}
		<footer>
			<button class="btn ghost" type="button" onclick={() => finish(null)}>{t('common.cancel')}</button>
			<button class="btn" class:primary={!request.danger} class:danger={request.danger} type="submit">
				{request.confirm}
			</button>
		</footer>
	</form>
</Modal>

<style>
	form {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 4px;
		margin-bottom: 4px;
	}

	footer {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 12px;
	}
</style>
