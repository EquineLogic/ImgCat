<script lang="ts">
	let { onDrop }: { onDrop: (files: FileList) => void } = $props();

	let isOver = $state(false);

	function onDragOver(e: DragEvent) {
		e.preventDefault();
		if (e.dataTransfer?.types.includes('Files')) {
			isOver = true;
		}
	}

	function onDragLeave() {
		isOver = false;
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		isOver = false;
		if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
			onDrop(e.dataTransfer.files);
		}
	}
</script>

<svelte:window ondragover={onDragOver} />

{#if isOver}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-[100] bg-tw-darkblue/80 backdrop-blur-md
		       flex flex-col items-center justify-center border-4 border-dashed border-tw-neon/50 m-4 rounded-3xl
		       transition-all duration-200"
		ondragover={onDragOver}
		ondragleave={onDragLeave}
		ondrop={handleDrop}
	>
		<div class="flex flex-col items-center gap-6 animate-bounce">
			<div class="w-24 h-24 rounded-full bg-tw-neon/20 flex items-center justify-center text-tw-neon shadow-[0_0_30px_rgba(0,245,255,0.3)]">
				<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-12 h-12">
					<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
					<polyline points="17 8 12 3 7 8" />
					<line x1="12" y1="3" x2="12" y2="15" />
				</svg>
			</div>
			<div class="text-center">
				<h2 class="text-3xl font-bold text-white mb-2">Drop to Upload</h2>
				<p class="text-tw-neon/70 font-medium">Release your images to add them to the library</p>
			</div>
		</div>
	</div>
{/if}
