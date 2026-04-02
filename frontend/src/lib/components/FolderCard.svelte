<script lang="ts">
	let { name, id } = $props<{ name: string; id: string }>();
</script>

<button
	class="group flex flex-col items-center gap-3 p-5 rounded-2xl
	       bg-white/5 border border-white/10
	       hover:bg-tw-purple/10 hover:border-tw-purple/30
	       cursor-pointer transition-all duration-200"
>
	<svg
		xmlns="http://www.w3.org/2000/svg"
		viewBox="0 0 24 24"
		fill="none"
		stroke="currentColor"
		stroke-width="1.5"
		stroke-linecap="round"
		stroke-linejoin="round"
		class="w-12 h-12 text-tw-yellow/60 group-hover:text-tw-yellow transition-colors duration-200"
	>
		<path d="M2 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z" />
	</svg>
	<span class="text-sm text-white/70 group-hover:text-white truncate max-w-full transition-colors duration-200">
		{name}
	</span>
</button>
