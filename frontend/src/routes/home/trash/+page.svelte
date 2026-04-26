<script lang="ts">
	import { onMount } from 'svelte';
	import FolderCard from '$lib/components/FolderCard.svelte';
	import ImageCard from '$lib/components/ImageCard.svelte';
	import FileGrid from '$lib/components/FileGrid.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import { op } from '$lib/api';

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
		try {
			const r = await op<{ op: 'TrashItems'; items: TrashEntry[] }>({ op: 'ListTrash' });
			items = r.items;
		} catch {}
		loading = false;
	}

	async function restore(id: string) {
		error = '';
		try {
			await op({ op: 'RestoreEntry', id });
		} catch (e: any) {
			error = e?.message || 'Restore failed';
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
		try {
			await op({ op: 'DeleteTrashEntry', id });
		} catch (e: any) {
			error = e?.message || 'Delete failed';
			return;
		}
		await load();
	}

	let menuOpen = $state(false);
	let menuX = $state(0);
	let menuY = $state(0);
	let menuTarget = $state<TrashEntry | null>(null);

	const menuItems = $derived(
		menuTarget
			? [
					{
						label: 'Restore',
						icon: 'folder-open',
						action: () => menuTarget && restore(menuTarget.id)
					},
					{
						label: 'Delete forever',
						icon: 'delete',
						danger: true,
						action: () => menuTarget && askPurge(menuTarget)
					}
				]
			: []
	);

	function openMenu(e: MouseEvent, entry: TrashEntry) {
		e.preventDefault();
		menuTarget = entry;
		menuX = e.clientX;
		menuY = e.clientY;
		menuOpen = true;
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

	<p class="mb-4 text-xs text-white/40">
		Right-click (desktop) or long-press (mobile) an item to restore or permanently delete it.
	</p>

	{#if error}
		<div class="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-sm text-red-300">
			{error}
		</div>
	{/if}

	<FileGrid {folders} {files} {loading} {folderItem} {fileItem} />
</div>

{#snippet folderItem(folder: TrashEntry)}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="opacity-70 hover:opacity-100 transition-opacity duration-150"
		oncontextmenu={(e) => openMenu(e, folder)}
	>
		<div class="pointer-events-none">
			<FolderCard name={folder.name} id={folder.id} readonly />
		</div>
	</div>
{/snippet}

{#snippet fileItem(file: TrashEntry)}
	{#if file.url}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="opacity-70 hover:opacity-100 transition-opacity duration-150"
			oncontextmenu={(e) => openMenu(e, file)}
		>
			<div class="pointer-events-none">
				<ImageCard name={file.name} id={file.id} url={file.url} readonly />
			</div>
		</div>
	{/if}
{/snippet}

<ContextMenu bind:open={menuOpen} x={menuX} y={menuY} items={menuItems} />

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
