<script lang="ts">
	type MenuItem = { label: string; icon: string; danger?: boolean; action: () => void };

	let { open = $bindable(false), x = 0, y = 0, items = [] } = $props<{
		open: boolean;
		x: number;
		y: number;
		items: MenuItem[];
	}>();

	function handleClick(action: () => void) {
		action();
		open = false;
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') open = false;
	}
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="fixed inset-0 z-50" onmousedown={() => (open = false)}>
		<div
			class="absolute flex flex-col py-1.5 min-w-[160px]
			       bg-tw-darkblue/90 backdrop-blur-xl border border-white/10
			       rounded-xl shadow-[0_8px_30px_rgba(0,0,0,0.4)] overflow-hidden"
			style="left: {x}px; top: {y}px"
			onmousedown={(e) => e.stopPropagation()}
		>
			{#each items as item}
				<button
					onclick={() => handleClick(item.action)}
					class="flex items-center gap-2.5 px-3.5 py-2 text-sm
					       transition-colors duration-150 cursor-pointer
					       {item.danger
						? 'text-red-400 hover:bg-red-400/10'
						: 'text-white/70 hover:text-white hover:bg-white/10'}"
				>
					{#if item.icon === 'rename'}
						<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" class="w-4 h-4">
							<path d="M17 3a2.83 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
							<path d="m15 5 4 4" />
						</svg>
					{:else if item.icon === 'delete'}
						<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" class="w-4 h-4">
							<path d="M3 6h18" />
							<path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
							<path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
						</svg>
					{/if}
					{item.label}
				</button>
			{/each}
		</div>
	</div>
{/if}
