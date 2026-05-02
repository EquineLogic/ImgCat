<script lang="ts">
	import { onMount } from 'svelte';
	import FolderCard from '$lib/components/FolderCard.svelte';
	import ImageCard from '$lib/components/ImageCard.svelte';
	import FileGrid from '$lib/components/FileGrid.svelte';
	import { folders, fetchFolders, currentFolderId, breadcrumbs, navigateToBreadcrumb, resetToRoot } from '$lib/stores/folders';
	import { files, fetchFiles } from '$lib/stores/files';
	import { user, updatePreference } from '$lib/stores/auth';
	import { op } from '$lib/api';

	let loading = $state(true);
	let editMode = $state(false);
	let selectedIds = $state(new Set<string>());

	$effect(() => {
		if (!editMode) {
			selectedIds = new Set();
		}
	});

	// Drag state
	let dragging = $state(false);
	let dragType: 'folder' | 'file' | null = $state(null);
	let dragId: string | null = $state(null);
	let dragEl: HTMLElement | null = $state(null);
	let dragClone: HTMLElement | null = $state(null);
	let dropTargetFolderId: string | null | undefined = $state(undefined);
	let startX = 0;
	let startY = 0;
	let offsetX = 0;
	let offsetY = 0;

	// Breadcrumb resize state
	let resizingBreadcrumb = $state(false);
	let initialBreadcrumbSize = 0;
	let startDragY = 0;

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
		if (resizingBreadcrumb) {
			const dy = e.clientY - startDragY;
			const newSize = Math.max(10, Math.min(48, initialBreadcrumbSize + dy));
			// Update local state immediately for smooth UI
			if ($user) {
				$user.preferences.breadcrumb_size = newSize;
			}
			return;
		}

		if (!dragId || !dragEl) return;

		// Start dragging after moving a few pixels (prevents accidental drags)
		if (!dragging) {
			const dx = e.clientX - startX;
			const dy = e.clientY - startY;
			if (Math.abs(dx) < 5 && Math.abs(dy) < 5) return;

			dragging = true;

			// If dragging an item that isn't part of current selection, make it the only selected item
			if (!selectedIds.has(dragId)) {
				selectedIds = new Set([dragId]);
			}

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

			// Add a badge if multiple items are being moved
			if (selectedIds.size > 1) {
				const badge = document.createElement('div');
				badge.className = 'absolute -top-3 -right-3 bg-tw-neon text-tw-darkblue w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold shadow-lg border-2 border-tw-darkblue';
				badge.textContent = selectedIds.size.toString();
				dragClone.appendChild(badge);
			}

			document.body.appendChild(dragClone);
		}

		if (dragClone) {
			dragClone.style.left = (e.clientX - offsetX) + 'px';
			dragClone.style.top = (e.clientY - offsetY) + 'px';
		}

		// Check whether we're hovering a folder (move target).
		let hoveredFolder: string | null | undefined = undefined;
		
		// Use elementFromPoint for much better performance than iterating all rects
		const elementUnderCursor = document.elementFromPoint(e.clientX, e.clientY);
		if (elementUnderCursor) {
			const targetEl = elementUnderCursor.closest('[data-type="folder"]');
			if (targetEl) {
				const candidateId = targetEl.getAttribute('data-id');
				// candidateId might be null or empty string if dropping to root
				const normalizedId = candidateId === '' ? null : candidateId;
				if (normalizedId !== $currentFolderId && !selectedIds.has(normalizedId ?? '')) {
					hoveredFolder = normalizedId;
				}
			}
		}

		dropTargetFolderId = hoveredFolder;
		// Skip reorder logic while hovering a folder or if moving multiple items
		if (dropTargetFolderId !== undefined || selectedIds.size > 1) return;

		// Find which item the cursor is over and reorder
		const list = dragType === 'folder' ? $folders : $files;
		const containerSelector = dragType === 'folder'
			? '[data-droppable="folders"]'
			: '[data-droppable="files"]';

		if (elementUnderCursor) {
			const targetEl = elementUnderCursor.closest(`[data-type="${dragType}"]`);
			if (targetEl && targetEl.parentElement?.closest(containerSelector)) {
				const targetId = targetEl.getAttribute('data-id');
				if (targetId && targetId !== dragId) {
					const fromIndex = list.findIndex((item) => item.id === dragId);
					const toIndex = list.findIndex((item) => item.id === targetId);

					if (fromIndex !== -1 && toIndex !== -1) {
						// Only reorder if we've moved past the center of the target
						const rect = targetEl.getBoundingClientRect();
						const isAfter = toIndex > fromIndex;
						const threshold = isAfter 
							? e.clientX > rect.left + rect.width * 0.5 
							: e.clientX < rect.left + rect.width * 0.5;

						if (threshold) {
							const newList = [...list];
							const [item] = newList.splice(fromIndex, 1);
							newList.splice(toIndex, 0, item);

							if (dragType === 'folder') {
								folders.set(newList);
							} else {
								files.set(newList as any);
							}
						}
					}
				}
			}
		}
	}

	function onPointerUp() {
		if (resizingBreadcrumb) {
			resizingBreadcrumb = false;
			if ($user) {
				updatePreference('breadcrumb_size', $user.preferences.breadcrumb_size);
			}
			return;
		}

		if (!dragId) return;

		if (dragging) {
			if (dropTargetFolderId !== undefined) {
				moveEntries(Array.from(selectedIds), dropTargetFolderId);
			} else if (selectedIds.size === 1) {
				if (dragType === 'folder') {
					persistOrder($folders.map((f) => f.id));
				} else if (dragType === 'file') {
					persistOrder($files.map((f) => f.id));
				}
			}
		} else {
			// Toggle selection if we just clicked
			if (selectedIds.has(dragId)) {
				selectedIds.delete(dragId);
			} else {
				selectedIds.add(dragId);
			}
			// Force refresh since Svelte 5 Set state needs re-assignment or fine-grained reactivity
			selectedIds = new Set(selectedIds);
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
		dropTargetFolderId = undefined;
	}

	async function persistOrder(ids: string[]) {
		try {
			await op({ op: 'Reorder', ids });
		} catch {}
	}

	async function moveEntries(ids: string[], parentId: string | null) {
		try {
			await op({ op: 'MoveEntries', ids, parent_id: parentId });
			selectedIds.clear();
			selectedIds = new Set();
			await Promise.all([fetchFolders(), fetchFiles()]);
		} catch {}
	}

	function startResize(e: PointerEvent) {
		e.preventDefault();
		resizingBreadcrumb = true;
		initialBreadcrumbSize = $user?.preferences.breadcrumb_size ?? 14;
		startDragY = e.clientY;
	}

</script>

<svelte:window onpointermove={onPointerMove} onpointerup={onPointerUp} />

<div class="p-8 select-none">
	<div class="flex items-center justify-between mb-6">
		<!-- Breadcrumbs -->
		{#if $breadcrumbs.length > 1}
			<div class="group/breadcrumb relative flex flex-col gap-1">
				<nav class="flex items-center gap-1.5" style="font-size: {$user?.preferences.breadcrumb_size ?? 14}px">
					{#each $breadcrumbs as crumb, i}
						{#if i > 0}
							<span class="text-white/20">/</span>
						{/if}
						{#if i < $breadcrumbs.length - 1}
							<button
								onclick={() => navigateToBreadcrumb(i)}
								data-type="folder"
								data-id={crumb.id ?? ''}
								class="px-2 py-1 rounded-lg transition-all duration-150 cursor-pointer
								       {dragging && dropTargetFolderId === (crumb.id ?? null) 
								       ? 'bg-tw-neon text-tw-darkblue font-bold shadow-[0_0_12px_rgba(0,245,255,0.4)] scale-110' 
								       : 'text-white/40 hover:text-tw-neon hover:bg-white/5'}"
							>
								{crumb.name}
							</button>
						{:else}
							<span class="text-white px-2 py-1">{crumb.name}</span>
						{/if}
					{/each}
				</nav>
				<!-- Resize handle -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div 
					onpointerdown={startResize}
					class="h-1 w-full bg-tw-neon/0 hover:bg-tw-neon/30 active:bg-tw-neon/50 cursor-ns-resize transition-colors duration-150 rounded-full"
				></div>
			</div>
		{:else}
			<div class="group/breadcrumb relative flex flex-col gap-1">
				<div class="h-6"></div> <!-- spacer -->
				<!-- Resize handle -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div 
					onpointerdown={startResize}
					class="h-1 w-24 bg-tw-neon/0 hover:bg-tw-neon/30 active:bg-tw-neon/50 cursor-ns-resize transition-colors duration-150 rounded-full"
				></div>
			</div>
		{/if}

		<!-- Edit mode toggle (desktop only) -->
		<div class="flex items-center gap-3">
			{#if selectedIds.size > 0}
				<div class="px-3 py-1.5 rounded-lg bg-tw-purple/20 border border-tw-purple/30 text-tw-purple-light text-sm font-medium animate-in fade-in slide-in-from-right-4 duration-200">
					{selectedIds.size} {selectedIds.size === 1 ? 'item' : 'items'} selected
				</div>
			{/if}

			<button
				onclick={() => (editMode = !editMode)}
				class="hidden md:block px-3 py-1.5 rounded-lg text-sm font-medium transition-colors duration-150 cursor-pointer
				       {editMode
						? 'bg-tw-neon text-tw-darkblue hover:bg-tw-neon/90'
						: 'bg-white/5 text-white/60 hover:text-white hover:bg-white/10'}"
			>
				{editMode ? 'Done' : 'Edit'}
			</button>
		</div>
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
		onclickcapture={(e) => { if (editMode) { e.preventDefault(); e.stopPropagation(); } }}
		data-id={folder.id}
		data-type="folder"
		class="transition-all duration-150 rounded-xl relative group/item
		       {editMode ? 'cursor-grab active:cursor-grabbing' : ''}
		       {dragging && selectedIds.has(folder.id) ? 'opacity-30' : ''}
		       {selectedIds.has(folder.id) ? 'ring-2 ring-tw-neon bg-tw-neon/10 scale-[1.02]' : ''}
		       {dragging && dropTargetFolderId === folder.id ? 'ring-2 ring-tw-neon scale-105 bg-tw-neon/20' : ''}"
	>
		<div class={editMode ? 'pointer-events-none' : ''}>
			<FolderCard name={folder.name} id={folder.id} />
		</div>

		{#if selectedIds.has(folder.id)}
			<div class="absolute top-2 right-2 w-5 h-5 bg-tw-neon text-tw-darkblue rounded-full flex items-center justify-center shadow-lg animate-in zoom-in duration-200">
				<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3.5 h-3.5">
					<path fill-rule="evenodd" d="M16.704 4.153a.75.75 0 01.143 1.052l-8 10.5a.75.75 0 01-1.127.075l-4.5-4.5a.75.75 0 011.06-1.06l3.894 3.893 7.48-9.817a.75.75 0 011.05-.143z" clip-rule="evenodd" />
				</svg>
			</div>
		{/if}
	</div>
{/snippet}

{#snippet fileItem(file: { id: string; name: string; url: string })}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		onpointerdown={(e) => onPointerDown(e, file.id, 'file')}
		onclickcapture={(e) => { if (editMode) { e.preventDefault(); e.stopPropagation(); } }}
		data-id={file.id}
		data-type="file"
		class="transition-all duration-150 rounded-2xl relative group/item
		       {editMode ? 'cursor-grab active:cursor-grabbing' : ''}
		       {dragging && selectedIds.has(file.id) ? 'opacity-30' : ''}
		       {selectedIds.has(file.id) ? 'ring-2 ring-tw-neon bg-tw-neon/10 scale-[1.02]' : ''}"
	>
		<div class={editMode ? 'pointer-events-none' : ''}>
			<ImageCard name={file.name} id={file.id} url={file.url} />
		</div>

		{#if selectedIds.has(file.id)}
			<div class="absolute top-2 right-2 w-6 h-6 bg-tw-neon text-tw-darkblue rounded-full flex items-center justify-center shadow-lg animate-in zoom-in duration-200">
				<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
					<path fill-rule="evenodd" d="M16.704 4.153a.75.75 0 01.143 1.052l-8 10.5a.75.75 0 01-1.127.075l-4.5-4.5a.75.75 0 011.06-1.06l3.894 3.893 7.48-9.817a.75.75 0 011.05-.143z" clip-rule="evenodd" />
				</svg>
			</div>
		{/if}
	</div>
{/snippet}
