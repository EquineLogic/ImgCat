<script lang="ts">
	import ContextMenu from './ContextMenu.svelte';
	import RenameModal from './RenameModal.svelte';
	import { fetchFolders, openFolder } from '$lib/stores/folders';

	let { name, id } = $props<{ name: string; id: string }>();

	function handleClick() {
		openFolder(id, name);
	}

	let menuOpen = $state(false);
	let menuX = $state(0);
	let menuY = $state(0);
	let showRename = $state(false);

	function onContextMenu(e: MouseEvent) {
		e.preventDefault();
		menuX = e.clientX;
		menuY = e.clientY;
		menuOpen = true;
	}

	async function submitRename(newName: string): Promise<string | null> {
		const res = await fetch(`http://localhost:3000/rename_folder`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({ id, name: newName })
		});
		if (!res.ok) return await res.text();
		await fetchFolders();
		return null;
	}

	async function deleteFolder() {
		const res = await fetch(`http://localhost:3000/delete_folder`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({ id })
		});
		if (res.ok) {
			await fetchFolders();
		}
	}

	const menuItems = [
		{
			label: 'Rename',
			icon: 'rename',
			action: () => (showRename = true)
		},
		{
			label: 'Delete',
			icon: 'delete',
			danger: true,
			action: deleteFolder
		}
	];
</script>

<button
	onclick={handleClick}
	oncontextmenu={onContextMenu}
	class="group flex items-center gap-3 px-4 py-3 rounded-xl
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
		class="w-6 h-6 shrink-0 text-tw-yellow/60 group-hover:text-tw-yellow transition-colors duration-200"
	>
		<path d="M2 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z" />
	</svg>
	<span class="text-sm text-white/70 group-hover:text-white truncate transition-colors duration-200">
		{name}
	</span>
</button>

<ContextMenu bind:open={menuOpen} x={menuX} y={menuY} items={menuItems} />

<RenameModal bind:open={showRename} title="Rename Folder" currentName={name} onSubmit={submitRename} />
