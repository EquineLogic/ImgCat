<script lang="ts">
	import Sidebar from '$lib/components/Sidebar.svelte';

	let { children } = $props();
</script>

<div class="min-h-screen bg-tw-darkblue flex">
	<Sidebar mode="settings" />

	<!-- Main content -->
	<main class="flex-1 min-h-screen overflow-y-auto">
		{@render children()}
	</main>
</div>
