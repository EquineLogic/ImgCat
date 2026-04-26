<script lang="ts">
	import ContextMenu from './ContextMenu.svelte';
	import RenameModal from './RenameModal.svelte';
	import ImageViewer from './ImageViewer.svelte';
	import { fetchFiles } from '$lib/stores/files';
	import { fetchClient, op } from '$lib/api';

	let { name, id, url, readonly = false } = $props<{ name: string; id: string; url: string; readonly?: boolean }>();
	let viewerOpen = $state(false);

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
		try {
			await op({ op: 'RenameFile', id, name: newName });
		} catch (e: any) {
			return e?.message || 'Rename failed';
		}
		await fetchFiles();
		return null;
	}

	async function deleteFile() {
		try {
			await op({ op: 'DeleteFile', id });
			await fetchFiles();
		} catch {}
	}

	async function downloadFile() {
		const res = await fetchClient(url);
		if (!res.ok) return;
		const blob = await res.blob();
		const a = document.createElement('a');
		a.href = URL.createObjectURL(blob);
		a.download = name;
		document.body.appendChild(a);
		a.click();
		a.remove();
		URL.revokeObjectURL(a.href);
	}

	const menuItems = $derived(readonly
		? [
				{
					label: 'Download',
					icon: 'download',
					action: downloadFile
				}
			]
		: [
				{
					label: 'Rename',
					icon: 'rename',
					action: () => (showRename = true)
				},
				{
					label: 'Delete',
					icon: 'delete',
					danger: true,
					action: deleteFile
				}
			]);
</script>

<button
	type="button"
	onclick={() => (viewerOpen = true)}
	oncontextmenu={onContextMenu}
	class="group flex flex-col items-center gap-3 p-3 rounded-2xl
	       bg-white/5 border border-white/10
	       hover:bg-tw-purple/10 hover:border-tw-purple/30
	       cursor-pointer transition-all duration-200 text-left w-full"
>
	<img
		src={url}
		alt={name}
		class="w-full aspect-square object-cover rounded-xl pointer-events-none"
	/>
	<span
		class="text-sm text-white/70 group-hover:text-white truncate max-w-full transition-colors duration-200"
	>
		{name}
	</span>
</button>

<ContextMenu bind:open={menuOpen} x={menuX} y={menuY} items={menuItems} />

<RenameModal bind:open={showRename} title="Rename File" currentName={name} onSubmit={submitRename} />

<ImageViewer bind:open={viewerOpen} {id} {name} {url} {readonly} />
