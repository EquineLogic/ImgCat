<script lang="ts">
	import { onMount } from 'svelte';
	import FolderCard from '$lib/components/FolderCard.svelte';
	import ImageCard from '$lib/components/ImageCard.svelte';
	import FileGrid from '$lib/components/FileGrid.svelte';
	import { folders, fetchFolders, breadcrumbs, navigateToBreadcrumb, resetToRoot } from '$lib/stores/folders';
	import { files, fetchFiles } from '$lib/stores/files';
	import { API_BASE } from '$lib/config';

	let loading = $state(true);
	let editMode = $state(false);

	// Drag state
	let dragging = $state(false);
	let dragType: 'folder' | 'file' | null = $state(null);
	let dragId: string | null = $state(null);
	let dragEl: HTMLElement | null = $state(null);
	let dragClone: HTMLElement | null = $state(null);
	let dropTargetFolderId: string | null = $state(null);
	let startX = 0;
	let startY = 0;
	let offsetX = 0;
	let offsetY = 0;

	onMount(async () => {
		resetToRoot();
		await Promise.all([fetchFolders(), fetchFiles()]);
		loading = false;
	});

	function onPointerDown(e: PointerEvent, id: string, type: 'folder' | 'file') {
		// Only allow dragging in edit mode, and only left click
		if (!editMode || e.button !== 0) return;
		const target = (e.currentTarget as HTMLElement);

		dragId = id;
		dragType = type;
		dragEl = target;
		startX = e.clientX;
		startY = e.clientY;

		const rect = target.getBoundingClientRect();
		offsetX = e.clientX - rect.left;
		offsetY = e.clientY - rect.top;
	}

	function onPointerMove(e: PointerEvent) {
		if (!dragId || !dragEl) return;

		// Start dragging after moving a few pixels (prevents accidental drags)
		if (!dragging) {
			const dx = e.clientX - startX;
			const dy = e.clientY - startY;
			if (Math.abs(dx) < 5 && Math.abs(dy) < 5) return;

			dragging = true;

			// Create a floating clone of the dragged element
			const rect = dragEl.getBoundingClientRect();
			dragClone = dragEl.cloneNode(true) as HTMLElement;
			dragClone.style.position = 'fixed';
			dragClone.style.width = rect.width + 'px';
			dragClone.style.height = rect.height + 'px';
			dragClone.style.zIndex = '1000';
			dragClone.style.pointerEvents = 'none';
			dragClone.style.opacity = '0.9';
			dragClone.style.transform = 'scale(1.05)';
			dragClone.style.transition = 'transform 0.15s, box-shadow 0.15s';
			dragClone.style.boxShadow = '0 12px 40px rgba(0,0,0,0.4)';
			dragClone.style.borderRadius = '12px';
			document.body.appendChild(dragClone);
		}

		if (dragClone) {
			dragClone.style.left = (e.clientX - offsetX) + 'px';
			dragClone.style.top = (e.clientY - offsetY) + 'px';
		}

		// Check whether we're hovering a folder (move target).
		// Works for both file and folder drags; a folder can't be dropped on itself.
		const folderContainer = document.querySelector('[data-droppable="folders"]');
		let hoveredFolder: string | null = null;
		if (folderContainer) {
			const folderChildren = Array.from(folderContainer.children) as HTMLElement[];
			for (let i = 0; i < folderChildren.length; i++) {
				const candidateId = $folders[i]?.id ?? null;
				if (candidateId === dragId) continue;
				const rect = folderChildren[i].getBoundingClientRect();
				if (
					e.clientX >= rect.left && e.clientX <= rect.right &&
					e.clientY >= rect.top && e.clientY <= rect.bottom
				) {
					hoveredFolder = candidateId;
					break;
				}
			}
		}
		dropTargetFolderId = hoveredFolder;
		// Skip reorder logic while hovering a folder so highlight stays visible
		if (dropTargetFolderId) return;

		// Find which item the cursor is over and reorder
		const list = dragType === 'folder' ? $folders : $files;
		const container = dragType === 'folder'
			? document.querySelector('[data-droppable="folders"]')
			: document.querySelector('[data-droppable="files"]');

		if (!container) return;

		const fromIndex = list.findIndex((item) => item.id === dragId);
		if (fromIndex === -1) return;

		const children = Array.from(container.children) as HTMLElement[];
		for (let i = 0; i < children.length; i++) {
			if (i === fromIndex) continue;
			const rect = children[i].getBoundingClientRect();
			const centerX = rect.left + rect.width / 2;
			const centerY = rect.top + rect.height / 2;

			// Only swap when the cursor has crossed past the center of the target
			// (prevents jitter when hovering near the edges)
			const crossedFromLeft = i > fromIndex && e.clientX > centerX && e.clientY >= rect.top && e.clientY <= rect.bottom;
			const crossedFromRight = i < fromIndex && e.clientX < centerX && e.clientY >= rect.top && e.clientY <= rect.bottom;
			const crossedFromAbove = i > fromIndex && e.clientY > centerY && e.clientX >= rect.left && e.clientX <= rect.right;
			const crossedFromBelow = i < fromIndex && e.clientY < centerY && e.clientX >= rect.left && e.clientX <= rect.right;

			if (crossedFromLeft || crossedFromRight || crossedFromAbove || crossedFromBelow) {
				const newList = [...list];
				const [item] = newList.splice(fromIndex, 1);
				newList.splice(i, 0, item);

				if (dragType === 'folder') {
					folders.set(newList);
				} else {
					files.set(newList as any);
				}
				break;
			}
		}
	}

	function onPointerUp() {
		if (!dragId) return;

		if (dragging) {
			if (dropTargetFolderId) {
				// Move the dragged item into the hovered folder
				moveEntry(dragId, dropTargetFolderId);
			} else if (dragType === 'folder') {
				persistOrder($folders.map((f) => f.id));
			} else if (dragType === 'file') {
				persistOrder($files.map((f) => f.id));
			}
		}

		// Cleanup
		if (dragClone) {
			dragClone.remove();
			dragClone = null;
		}
		dragging = false;
		dragId = null;
		dragType = null;
		dragEl = null;
		dropTargetFolderId = null;
	}

	async function persistOrder(ids: string[]) {
		await fetch(`${API_BASE}/reorder`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({ ids })
		});
	}

	async function moveEntry(id: string, parentId: string) {
		const res = await fetch(`${API_BASE}/move`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({ id, parent_id: parentId })
		});
		if (res.ok) {
			// Moved item is no longer in the current folder; refresh both lists
			await Promise.all([fetchFolders(), fetchFiles()]);
		}
	}
</script>

<svelte:window onpointermove={onPointerMove} onpointerup={onPointerUp} />

<div class="p-8 select-none">
	<div class="flex items-center justify-between mb-6">
		<!-- Breadcrumbs -->
		{#if $breadcrumbs.length > 1}
			<nav class="flex items-center gap-1.5 text-sm">
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
		{:else}
			<div></div>
		{/if}

		<!-- Edit mode toggle -->
		<button
			onclick={() => (editMode = !editMode)}
			class="px-3 py-1.5 rounded-lg text-sm font-medium transition-colors duration-150 cursor-pointer
			       {editMode
					? 'bg-tw-neon text-tw-darkblue hover:bg-tw-neon/90'
					: 'bg-white/5 text-white/60 hover:text-white hover:bg-white/10'}"
		>
			{editMode ? 'Done' : 'Edit'}
		</button>
	</div>

	<FileGrid
		folders={$folders}
		files={$files}
		{loading}
		foldersDroppable="folders"
		filesDroppable="files"
		folderItem={folderItem}
		fileItem={fileItem}
	/>
</div>

{#snippet folderItem(folder: { id: string; name: string })}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		onpointerdown={(e) => onPointerDown(e, folder.id, 'folder')}
		class="transition-transform duration-150 rounded-xl
		       {editMode ? 'cursor-grab active:cursor-grabbing' : ''}
		       {dragging && dragId === folder.id ? 'opacity-30' : ''}
		       {dropTargetFolderId === folder.id ? 'ring-2 ring-tw-neon scale-105' : ''}"
	>
		<div class={editMode ? 'pointer-events-none' : ''}>
			<FolderCard name={folder.name} id={folder.id} />
		</div>
	</div>
{/snippet}

{#snippet fileItem(file: { id: string; name: string; url: string })}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		onpointerdown={(e) => onPointerDown(e, file.id, 'file')}
		class="{editMode ? 'cursor-grab active:cursor-grabbing' : ''}
		       {dragging && dragId === file.id ? 'opacity-30' : ''}"
	>
		<div class={editMode ? 'pointer-events-none' : ''}>
			<ImageCard name={file.name} id={file.id} url={file.url} />
		</div>
	</div>
{/snippet}
