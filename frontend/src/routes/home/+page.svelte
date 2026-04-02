<script lang="ts">
	import { onMount } from 'svelte';
	import FolderCard from '$lib/components/FolderCard.svelte';
	import { folders, fetchFolders, breadcrumbs, navigateToBreadcrumb } from '$lib/stores/folders';

	let loading = $state(true);

	onMount(async () => {
		await fetchFolders();
		loading = false;
	});
</script>

<div class="p-8">
	<!-- Breadcrumbs -->
	{#if $breadcrumbs.length > 1}
		<nav class="flex items-center gap-1.5 mb-6 text-sm">
			{#each $breadcrumbs as crumb, i}
				{#if i > 0}
					<span class="text-white/20">/</span>
				{/if}
				{#if i < $breadcrumbs.length - 1}
					<button
						onclick={() => navigateToBreadcrumb(i)}
						class="text-white/40 hover:text-tw-neon cursor-pointer transition-colors duration-150"
					>
						{crumb.name}
					</button>
				{:else}
					<span class="text-white">{crumb.name}</span>
				{/if}
			{/each}
		</nav>
	{/if}

	{#if loading}
		<div class="flex justify-center mt-20">
			<div class="w-6 h-6 border-2 border-tw-neon/30 border-t-tw-neon rounded-full animate-spin"></div>
		</div>
	{:else if $folders.length === 0}
		<!-- Empty state -->
		<div class="w-full max-w-lg mx-auto mt-20 flex flex-col items-center gap-4 text-center">
			<div class="w-16 h-16 rounded-2xl bg-white/5 border border-white/10 flex items-center justify-center">
				<svg
					xmlns="http://www.w3.org/2000/svg"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="1.5"
					stroke-linecap="round"
					stroke-linejoin="round"
					class="w-8 h-8 text-white/20"
				>
					<path d="M2 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z" />
				</svg>
			</div>
			<p class="text-white/30 text-sm">This folder is empty.</p>
		</div>
	{:else}
		<div class="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-4">
			{#each $folders as folder (folder.id)}
				<FolderCard name={folder.name} id={folder.id} />
			{/each}
		</div>
	{/if}
</div>
