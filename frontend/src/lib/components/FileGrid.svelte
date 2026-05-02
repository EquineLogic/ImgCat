<script lang="ts" generics="F extends { id: string }, I extends { id: string }">
	import type { Snippet } from 'svelte';
	import { flip } from 'svelte/animate';

	let {
		folders,
		files,
		loading = false,
		foldersDroppable,
		filesDroppable,
		folderItem,
		fileItem
	} = $props<{
		folders: F[];
		files: I[];
		loading?: boolean;
		foldersDroppable?: string;
		filesDroppable?: string;
		folderItem: Snippet<[F]>;
		fileItem: Snippet<[I]>;
	}>();
</script>

{#if loading}
	<div class="flex justify-center mt-20">
		<div class="w-6 h-6 border-2 border-tw-neon/30 border-t-tw-neon rounded-full animate-spin"></div>
	</div>
{:else if folders.length === 0 && files.length === 0}
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
		<p class="text-white/30 text-sm">Nothing here.</p>
	</div>
{:else}
	{#if folders.length > 0}
		<div class="flex flex-wrap gap-2 mb-6" data-droppable={foldersDroppable}>
			{#if folders.length < 100}
				{#each folders as folder (folder.id)}
					<div animate:flip={{ duration: 200 }}>
						{@render folderItem(folder)}
					</div>
				{/each}
			{:else}
				{#each folders as folder (folder.id)}
					<div>
						{@render folderItem(folder)}
					</div>
				{/each}
			{/if}
		</div>
	{/if}

	{#if files.length > 0}
		<div class="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-4" data-droppable={filesDroppable}>
			{#if files.length < 100}
				{#each files as file (file.id)}
					<div animate:flip={{ duration: 200 }}>
						{@render fileItem(file)}
					</div>
				{/each}
			{:else}
				{#each files as file (file.id)}
					<div>
						{@render fileItem(file)}
					</div>
				{/each}
			{/if}
		</div>
	{/if}
{/if}
