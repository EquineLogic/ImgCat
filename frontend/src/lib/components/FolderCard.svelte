<script lang="ts">
	import { fetchFolders, openFolder } from '$lib/stores/folders';
	import { op } from '$lib/api';
	import { openContextMenu, openRenameModal } from '$lib/stores/ui';

	let { name, id, readonly = false } = $props<{ name: string; id: string; readonly?: boolean }>();

	function handleClick() {
		openFolder(id, name);
	}

	function onContextMenu(e: MouseEvent) {
		e.preventDefault();
		openContextMenu(e.clientX, e.clientY, menuItems);
	}

	async function submitRename(newName: string): Promise<string | null> {
		try {
			await op({ op: 'RenameFolder', id, name: newName });
		} catch (e: any) {
			return e?.message || 'Rename failed';
		}
		await fetchFolders();
		return null;
	}

	async function deleteFolder() {
		try {
			await op({ op: 'DeleteFolder', id });
			await fetchFolders();
		} catch {}
	}

	const menuItems = $derived(readonly
		? []
		: [
				{
					label: 'Rename',
					icon: 'rename',
					action: () => openRenameModal('Rename Folder', name, submitRename)
				},
				{
					label: 'Delete',
					icon: 'delete',
					danger: true,
					action: deleteFolder
				}
			]);
</script>

<button
	onclick={handleClick}
	oncontextmenu={onContextMenu}
	class="group flex items-center gap-3 px-4 py-3 rounded-xl
	       bg-white/5 border border-white/10
	       hover:bg-tw-purple/10 hover:border-tw-purple/30
	       cursor-pointer transition-all duration-200 w-full"
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
