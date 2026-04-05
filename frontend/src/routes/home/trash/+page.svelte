<script lang="ts">
	import { onMount } from 'svelte';
	import FolderCard from '$lib/components/FolderCard.svelte';
	import ImageCard from '$lib/components/ImageCard.svelte';
	import FileGrid from '$lib/components/FileGrid.svelte';
	import Modal from '$lib/components/Modal.svelte';

	type TrashEntry = {
		id: string;
		name: string;
		kind: 'folder' | 'file_link';
		mime_type: string | null;
		url?: string;
		deleted_at: string;
	};

	let items = $state<TrashEntry[]>([]);
	let loading = $state(true);
	let error = $state('');

	const folders = $derived(items.filter((i) => i.kind === 'folder'));
	const files = $derived(items.filter((i) => i.kind === 'file_link'));

	async function load() {
		loading = true;
		const res = await fetch('http://localhost:3000/list_trash', { credentials: 'include' });
		if (res.ok) items = await res.json();
		loading = false;
	}

	async function restore(id: string) {
		error = '';
		const res = await fetch('http://localhost:3000/restore', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({ id })
		});
		if (!res.ok) {
			error = await res.text();
			return;
		}
		await load();
	}

	let purgeModalOpen = $state(false);
	let purgeTarget = $state<TrashEntry | null>(null);

	function askPurge(entry: TrashEntry) {
		purgeTarget = entry;
		purgeModalOpen = true;
	}

	async function purge() {
		if (!purgeTarget) return;
		error = '';
		const id = purgeTarget.id;
		purgeModalOpen = false;
		const res = await fetch('http://localhost:3000/delete_trash_entry', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({ id })
		});
		if (!res.ok) {
			error = await res.text();
			return;
		}
		await load();
	}

	onMount(load);
</script>

<div class="p-8">
	<div class="flex items-center justify-between mb-6">
		<h1 class="text-xl font-semibold text-white">Trash</h1>
		<span class="text-sm text-white/40">
			{items.length} {items.length === 1 ? 'item' : 'items'}
		</span>
	</div>

	{#if error}
		<div class="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-sm text-red-300">
			{error}
		</div>
	{/if}

	<FileGrid {folders} {files} {loading} {folderItem} {fileItem} />
</div>

{#snippet actionBtn(entry: TrashEntry, kind: 'restore' | 'purge', overlay: boolean)}
	<button
		onclick={(e) => {
			e.stopPropagation();
			e.preventDefault();
			if (kind === 'restore') restore(entry.id);
			else askPurge(entry);
		}}
		aria-label={kind === 'restore' ? 'Restore' : 'Delete forever'}
		title={kind === 'restore' ? 'Restore' : 'Delete forever'}
		class={overlay
			? `absolute top-2 ${kind === 'restore' ? 'right-11' : 'right-2'} w-8 h-8 flex items-center justify-center rounded-lg bg-tw-darkblue/80 backdrop-blur opacity-0 group-hover:opacity-100 cursor-pointer transition-all duration-150 ${kind === 'restore' ? 'text-tw-neon hover:bg-tw-neon/20' : 'text-red-400 hover:bg-red-400/20'}`
			: `ml-1 w-7 h-7 flex items-center justify-center rounded-lg cursor-pointer transition-colors duration-150 ${kind === 'restore' ? 'text-tw-neon/70 hover:text-tw-neon hover:bg-tw-neon/10' : 'text-red-400/70 hover:text-red-400 hover:bg-red-400/10'}`}
	>
		{#if kind === 'restore'}
			<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-4 h-4">
				<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
				<path d="M3 3v5h5" />
			</svg>
		{:else}
			<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-4 h-4">
				<path d="M3 6h18" />
				<path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
				<path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
			</svg>
		{/if}
	</button>
{/snippet}

{#snippet folderItem(folder: TrashEntry)}
	<div class="group relative opacity-60 hover:opacity-100 transition-opacity duration-150 flex items-center">
		<div class="pointer-events-none">
			<FolderCard name={folder.name} id={folder.id} />
		</div>
		{@render actionBtn(folder, 'restore', false)}
		{@render actionBtn(folder, 'purge', false)}
	</div>
{/snippet}

{#snippet fileItem(file: TrashEntry)}
	{#if file.url}
		<div class="group relative opacity-60 hover:opacity-100 transition-opacity duration-150">
			<div class="pointer-events-none">
				<ImageCard name={file.name} id={file.id} url={file.url} />
			</div>
			{@render actionBtn(file, 'restore', true)}
			{@render actionBtn(file, 'purge', true)}
		</div>
	{/if}
{/snippet}

<Modal bind:open={purgeModalOpen} title="Delete forever?">
	{#if purgeTarget}
		<div class="flex flex-col gap-4">
			<p class="text-sm text-white/70">
				Permanently delete
				<span class="text-white font-medium">{purgeTarget.name}</span>?
				{#if purgeTarget.kind === 'folder'}
					All contents will also be purged.
				{/if}
				This cannot be undone.
			</p>
			<div class="flex gap-2 justify-end">
				<button
					onclick={() => (purgeModalOpen = false)}
					class="px-4 py-2 rounded-lg text-sm text-white/60 hover:text-white hover:bg-white/10 cursor-pointer transition-colors duration-150"
				>
					Cancel
				</button>
				<button
					onclick={purge}
					class="px-4 py-2 rounded-lg text-sm font-semibold text-white bg-red-500/80 hover:bg-red-500 cursor-pointer transition-colors duration-150"
				>
					Delete Forever
				</button>
			</div>
		</div>
	{/if}
</Modal>
