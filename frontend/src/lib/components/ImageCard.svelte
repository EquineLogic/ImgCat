<script lang="ts">
	import ImageViewer from './ImageViewer.svelte';

	let { name, id } = $props<{ name: string; id: string }>();
	let viewerOpen = $state(false);
</script>

<button
	type="button"
	onclick={() => (viewerOpen = true)}
	class="group flex flex-col items-center gap-3 p-3 rounded-2xl
	       bg-white/5 border border-white/10
	       hover:bg-tw-purple/10 hover:border-tw-purple/30
	       cursor-pointer transition-all duration-200 text-left w-full"
>
	<img
		src="http://localhost:3000/files/{id}"
		alt={name}
		class="w-full aspect-square object-cover rounded-xl pointer-events-none"
	/>
	<span class="text-sm text-white/70 group-hover:text-white truncate max-w-full transition-colors duration-200">
		{name}
	</span>
</button>

<ImageViewer bind:open={viewerOpen} {id} {name} />
