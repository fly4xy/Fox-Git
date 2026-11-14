<script lang="ts">
	let { size = 20 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
	<path
		fill="var(--accent)"
		d="M3 3 L9.5 8.2 Q12 7.4 14.5 8.2 L21 3 L20.4 13 Q19.6 19 12 22 Q4.4 19 3.6 13 Z"
	/>
</svg>
