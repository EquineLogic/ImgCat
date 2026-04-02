<script lang="ts">
	let { open = $bindable(false), title = '', children } = $props();

	function onBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) open = false;
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') open = false;
	}
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
		onmousedown={onBackdropClick}
	>
		<div
			class="w-full max-w-md bg-tw-darkblue/80 backdrop-blur-xl border border-tw-purple/30
			       rounded-2xl p-6 shadow-[0_0_40px_rgba(144,97,194,0.15)]"
		>
			<div class="flex items-center justify-between mb-5">
				<h2 class="text-lg font-semibold text-white">{title}</h2>
				<button
					onclick={() => (open = false)}
					aria-label="Close"
					class="w-8 h-8 flex items-center justify-center rounded-lg
					       text-white/40 hover:text-white hover:bg-white/10
					       cursor-pointer transition-colors duration-150"
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
						class="w-4 h-4"
					>
						<path d="M18 6 6 18M6 6l12 12" />
					</svg>
				</button>
			</div>

			{@render children()}
		</div>
	</div>
{/if}
