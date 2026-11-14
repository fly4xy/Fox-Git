<script lang="ts">
	import { app } from '$lib/state.svelte';

	/**
	 * A border that can be dragged. `value` is in pixels, or a share of `span` pixels.
	 * The result goes to the settings, so every tab shares it.
	 */
	let {
		axis,
		value = $bindable(),
		min,
		max,
		span = () => 1
	}: {
		axis: 'x' | 'y';
		value: number;
		min: number;
		max: number | (() => number);
		span?: () => number;
	} = $props();

	function start(event: PointerEvent) {
		const handle = event.currentTarget as HTMLElement;
		const at = (e: PointerEvent) => (axis === 'x' ? e.clientX : e.clientY);
		const from = value;
		const origin = at(event);
		const scale = span();
		const limit = typeof max === 'number' ? max : max();
		const move = (e: PointerEvent) => {
			const next = from + (at(e) - origin) / scale;
			value = Math.max(min, Math.min(limit, Math.abs(scale) === 1 ? Math.round(next) : next));
		};
		handle.setPointerCapture(event.pointerId);
		handle.addEventListener('pointermove', move);
		handle.addEventListener(
			'pointerup',
			() => {
				handle.removeEventListener('pointermove', move);
				app.save();
			},
			{ once: true }
		);
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="splitter {axis}" onpointerdown={start}></div>

<style>
	.splitter {
		position: relative;
		z-index: 2;
		flex: none;
	}

	.splitter::before {
		content: '';
		position: absolute;
	}

	.splitter::after {
		content: '';
		position: absolute;
		background: var(--accent);
		opacity: 0;
		transition: opacity 140ms var(--ease);
	}

	.splitter:hover::after,
	.splitter:active::after {
		opacity: 1;
	}

	.x {
		width: 0;
		cursor: col-resize;
	}

	.x::before {
		inset: 0 -4px;
	}

	.x::after {
		top: 0;
		bottom: 0;
		left: -2px;
		width: 2px;
	}

	.y {
		height: 0;
		cursor: row-resize;
	}

	.y::before {
		inset: -4px 0;
	}

	.y::after {
		top: -2px;
		right: 0;
		left: 0;
		height: 2px;
	}
</style>
