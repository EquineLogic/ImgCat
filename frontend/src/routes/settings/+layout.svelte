<script lang="ts">
	import Sidebar from '$lib/components/Sidebar.svelte';

	let { children } = $props();
</script>

<div class="min-h-screen bg-tw-darkblue flex">
	<Sidebar mode="settings" />

	<!-- Main content -->
	<main class="flex-1 min-h-screen overflow-y-auto pt-14 md:pt-0">
		{@render children()}
	</main>
</div>
