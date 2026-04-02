<script lang="ts">
	import ContextMenu from './ContextMenu.svelte';
	import Modal from './Modal.svelte';
	import { fetchFolders, openFolder } from '$lib/stores/folders';

	let { name, id } = $props<{ name: string; id: string }>();

	function handleClick() {
		openFolder(id, name);
	}

	let menuOpen = $state(false);
	let menuX = $state(0);
	let menuY = $state(0);

	let showRename = $state(false);
	let newName = $state(name);
	let renameError = $state('');

	function onContextMenu(e: MouseEvent) {
		e.preventDefault();
		menuX = e.clientX;
		menuY = e.clientY;
		menuOpen = true;
	}

	async function renameFolder() {
		if (!newName.trim() || newName.trim() === name) {
			showRename = false;
			return;
		}
		const res = await fetch(`http://localhost:3000/rename_folder`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({ id, name: newName.trim() })
		});
		if (!res.ok) {
			renameError = await res.text();
			return;
		}
		renameError = '';
		showRename = false;
		await fetchFolders();
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
			action: () => {
				newName = name;
				renameError = '';
				showRename = true;
			}
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

<ContextMenu bind:open={menuOpen} x={menuX} y={menuY} items={menuItems} />

<Modal bind:open={showRename} title="Rename Folder">
	<form onsubmit={(e) => { e.preventDefault(); renameFolder(); }} class="flex flex-col gap-4">
		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">New Name</span>
			<input
				type="text"
				bind:value={newName}
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
				       border border-tw-purple/40 text-white
				       placeholder:text-white/30
				       focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>
		{#if renameError}
			<p class="text-sm text-red-400">{renameError}</p>
		{/if}
		<button
			type="submit"
			disabled={!newName.trim() || newName.trim() === name}
			class="rounded-lg py-2.5 font-semibold text-white
			       transition-colors duration-200
			       focus:outline-none focus:ring-2 focus:ring-tw-neon
			       {newName.trim() && newName.trim() !== name ? 'bg-tw-purple hover:bg-tw-pink cursor-pointer' : 'bg-white/10 text-white/30 cursor-not-allowed'}"
		>
			Rename
		</button>
	</form>
</Modal>
