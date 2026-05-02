<script lang="ts">
	import { fetchFiles } from '$lib/stores/files';
	import { fetchClient, op } from '$lib/api';
	import { openContextMenu, openRenameModal, openImageViewer } from '$lib/stores/ui';

	let { name, id, url, readonly = false } = $props<{ name: string; id: string; url: string; readonly?: boolean }>();

	function onContextMenu(e: MouseEvent) {
		e.preventDefault();
		openContextMenu(e.clientX, e.clientY, menuItems);
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
					action: () => openRenameModal('Rename File', name, submitRename)
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
	onclick={() => openImageViewer(id, name, url, readonly)}
	oncontextmenu={onContextMenu}
	class="group flex flex-col items-center gap-3 p-3 rounded-2xl
	       bg-white/5 border border-white/10
	       hover:bg-tw-purple/10 hover:border-tw-purple/30
	       cursor-pointer transition-all duration-200 text-left w-full"
>
	<img
		src={url}
		alt={name}
		loading="lazy"
		class="w-full aspect-square object-cover rounded-xl pointer-events-none"
	/>
	<span
		class="text-sm text-white/70 group-hover:text-white truncate max-w-full transition-colors duration-200"
	>
		{name}
	</span>
</button>
