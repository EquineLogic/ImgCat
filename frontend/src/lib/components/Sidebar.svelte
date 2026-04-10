<script lang="ts">
	import { user } from '$lib/stores/auth';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import Modal from './Modal.svelte';
	import { fetchFolders, currentFolderId, folders } from '$lib/stores/folders';
	import { fetchFiles, files } from '$lib/stores/files';
	import { pendingRequests, fetchPendingRequests, sendShareRequest } from '$lib/stores/sharing';
	import { onMount } from 'svelte';
	import { API_BASE } from '$lib/config';

	let { mode = 'home' }: { mode?: 'home' | 'settings' } = $props();

	const MIN_W = 72;
	const MAX_W = 240;
	const COLLAPSE_THRESHOLD = 110;

	let sidebarWidth = $state(220);
	let dragging = $state(false);
	let showNewFolder = $state(false);
	let folderName = $state('');
	let folderError = $state('');

	let showUpload = $state(false);
	let uploadFiles: File[] = $state([]);
	let uploadError = $state('');
	let uploading = $state(false);

	let showShareItem = $state(false);
	let shareUsername = $state('');
	let shareSelectedId = $state('');
	let shareError = $state('');
	let sharing = $state(false);

	const collapsed = $derived(sidebarWidth < COLLAPSE_THRESHOLD);
	const pendingCount = $derived($pendingRequests.length);

	onMount(() => {
		fetchPendingRequests();
	});

	const navItems = $derived(
		mode === 'home'
			? [
					{
						label: 'New Folder',
						icon: 'folder-plus',
						action: () => (showNewFolder = true)
					},
					{
						label: 'Upload Image',
						icon: 'image-plus',
						action: () => (showUpload = true)
					},
					{
						label: 'My Library',
						href: '/home',
						icon: 'library'
					},
					{
						label: 'Share Item',
						icon: 'share',
						action: () => (showShareItem = true)
					},
					{
						label: 'Shared with Me',
						href: '/home/shared',
						icon: 'share',
						badge: pendingCount > 0 ? pendingCount : undefined
					}
				]
			: [
					{
						label: 'Back to Library',
						href: '/home',
						icon: 'arrow-left'
					},
					{
						label: 'Profile',
						href: '/settings',
						icon: 'user'
					},
					{
						label: 'Trash Cleanup',
						href: '/settings/cleanup',
						icon: 'broom'
					}
				]
	);

	const bottomNavItems = $derived(
		mode === 'home'
			? [
					{
						label: 'Trash',
						href: '/home/trash',
						icon: 'trash'
					}
				]
			: []
	);

	async function createFolder() {
		if (!folderName.trim()) return;
		const res = await fetch(`${API_BASE}/create_folder`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({ name: folderName.trim(), parent_id: $currentFolderId })
		});
		if (!res.ok) {
			folderError = await res.text();
			return;
		}
		folderName = '';
		folderError = '';
		showNewFolder = false;
		await fetchFolders();
	}

	function onFileSelect(e: Event) {
		const input = e.target as HTMLInputElement;
		const files = input.files;
		if (files && files.length > 0) {
			uploadFiles = [...uploadFiles, ...Array.from(files)];
		}
		input.value = '';
	}

	function removeFile(index: number) {
		uploadFiles = uploadFiles.filter((_, i) => i !== index);
	}

	async function uploadImages() {
		if (uploadFiles.length === 0) return;
		uploading = true;
		uploadError = '';
		try {
			for (const file of uploadFiles) {
				const form = new FormData();
				form.append('file', file);
				form.append('name', file.name.replace(/\.[^.]+$/, ''));
				if ($currentFolderId) form.append('parent_id', $currentFolderId);
				const res = await fetch(`${API_BASE}/upload_file`, {
					method: 'POST',
					credentials: 'include',
					body: form
				});
				if (!res.ok) {
					uploadError = `Failed to upload ${file.name}: ${await res.text()}`;
					return;
				}
			}
			uploadFiles = [];
			uploadError = '';
			showUpload = false;
			await fetchFiles();
		} catch (e) {
			uploadError = 'Upload failed';
		} finally {
			uploading = false;
		}
	}

	const shareableItems = $derived([
		...$folders.map((f) => ({ id: f.id, name: f.name, type: 'folder' as const })),
		...$files.map((f) => ({ id: f.id, name: f.name, type: 'file' as const }))
	]);

	async function shareItem() {
		if (!shareSelectedId || !shareUsername.trim()) return;
		sharing = true;
		shareError = '';
		try {
			await sendShareRequest(shareSelectedId, shareUsername.trim());
			shareUsername = '';
			shareSelectedId = '';
			shareError = '';
			showShareItem = false;
		} catch (e: any) {
			shareError = e.message || 'Failed to share';
		} finally {
			sharing = false;
		}
	}

	function isActive(href: string) {
		return page.url.pathname === href;
	}

	function startDrag(e: MouseEvent) {
		e.preventDefault();
		dragging = true;
		const onMove = (ev: MouseEvent) => {
			sidebarWidth = Math.min(MAX_W, Math.max(MIN_W, ev.clientX));
		};
		const onUp = () => {
			dragging = false;
			if (sidebarWidth < COLLAPSE_THRESHOLD) {
				sidebarWidth = MIN_W;
			}
			window.removeEventListener('mousemove', onMove);
			window.removeEventListener('mouseup', onUp);
		};
		window.addEventListener('mousemove', onMove);
		window.addEventListener('mouseup', onUp);
	}

	async function handleSignOut() {
		await fetch(`${API_BASE}/signout`, {
			method: 'POST',
			credentials: 'include'
		});
		user.set(null);
		goto('/signin');
	}
</script>

{#snippet navIcon(icon: string)}
	{#if icon === 'folder-plus'}
		<svg
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="1.8"
			stroke-linecap="round"
			stroke-linejoin="round"
			class="w-5 h-5 shrink-0"
		>
			<path d="M12 10v6M9 13h6" />
			<path d="M2 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z" />
		</svg>
	{:else if icon === 'image-plus'}
		<svg
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="1.8"
			stroke-linecap="round"
			stroke-linejoin="round"
			class="w-5 h-5 shrink-0"
		>
			<rect x="3" y="3" width="18" height="18" rx="2" />
			<circle cx="8.5" cy="8.5" r="1.5" />
			<path d="M21 15l-5-5L5 21" />
			<line x1="16" y1="5" x2="16" y2="11" />
			<line x1="13" y1="8" x2="19" y2="8" />
		</svg>
	{:else if icon === 'trash'}
		<svg
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="1.8"
			stroke-linecap="round"
			stroke-linejoin="round"
			class="w-5 h-5 shrink-0"
		>
			<path d="M3 6h18" />
			<path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
			<path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
		</svg>
	{:else if icon === 'user'}
		<svg
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="1.8"
			stroke-linecap="round"
			stroke-linejoin="round"
			class="w-5 h-5 shrink-0"
		>
			<path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
			<circle cx="12" cy="7" r="4" />
		</svg>
	{:else if icon === 'broom'}
		<svg
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="1.8"
			stroke-linecap="round"
			stroke-linejoin="round"
			class="w-5 h-5 shrink-0"
		>
			<path d="M19.4 14.6 9.4 4.6a2 2 0 0 0-2.8 2.8l10 10" />
			<path d="M8 13 3 18l3 3 5-5" />
			<path d="M14 22h8" />
			<path d="M18 18l-4 4" />
		</svg>
	{:else if icon === 'arrow-left'}
		<svg
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="1.8"
			stroke-linecap="round"
			stroke-linejoin="round"
			class="w-5 h-5 shrink-0"
		>
			<path d="M19 12H5" />
			<path d="M12 19l-7-7 7-7" />
		</svg>
	{:else if icon === 'share'}
		<svg
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="1.8"
			stroke-linecap="round"
			stroke-linejoin="round"
			class="w-5 h-5 shrink-0"
		>
			<circle cx="18" cy="5" r="3" />
			<circle cx="6" cy="12" r="3" />
			<circle cx="18" cy="19" r="3" />
			<line x1="8.59" y1="13.51" x2="15.42" y2="17.49" />
			<line x1="15.41" y1="6.51" x2="8.59" y2="10.49" />
		</svg>
	{:else if icon === 'library'}
		<svg
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="1.8"
			stroke-linecap="round"
			stroke-linejoin="round"
			class="w-5 h-5 shrink-0"
		>
			<rect x="3" y="3" width="7" height="7" rx="1" />
			<rect x="14" y="3" width="7" height="7" rx="1" />
			<rect x="3" y="14" width="7" height="7" rx="1" />
			<rect x="14" y="14" width="7" height="7" rx="1" />
		</svg>
	{/if}
{/snippet}

{#if dragging}
	<div class="fixed inset-0 z-50 cursor-col-resize"></div>
{/if}

<aside
	class="h-screen sticky top-0 flex flex-col
	       bg-tw-darkblue border-r border-white/10 shrink-0 overflow-hidden
	       {dragging ? '' : 'transition-[width] duration-150'}"
	style="width: {sidebarWidth}px"
>
	<!-- Logo -->
	<div class="flex items-center gap-3 px-4 h-16 border-b border-white/10 shrink-0">
		<a
			href="/home"
			class="text-2xl font-extrabold leading-none whitespace-nowrap
			       bg-linear-to-r from-tw-purple to-tw-pink
			       bg-clip-text text-transparent no-underline"
		>
			{collapsed ? 'IC' : 'ImgCat'}
		</a>
	</div>

	<!-- Nav items -->
	<nav class="flex-1 flex flex-col gap-1 px-3 mt-4">
		{#each navItems as item}
			{#if item.action}
				<button
					onclick={item.action}
					class="group flex items-center gap-3 px-3 py-2.5 rounded-xl
					       transition-all duration-200 whitespace-nowrap overflow-hidden
					       text-white/50 hover:text-white hover:bg-white/5 cursor-pointer"
				>
					{@render navIcon(item.icon)}
					{#if !collapsed}
						<span class="text-sm font-medium truncate">{item.label}</span>
					{/if}
				</button>
			{:else}
				<a
					href={item.href}
					class="group flex items-center gap-3 px-3 py-2.5 rounded-xl
					       no-underline transition-all duration-200 whitespace-nowrap overflow-hidden
					       {isActive(item.href ?? '')
						? 'bg-tw-purple/20 text-tw-neon shadow-[inset_0_0_12px_rgba(0,245,255,0.08)]'
						: 'text-white/50 hover:text-white hover:bg-white/5'}"
				>
					{@render navIcon(item.icon)}
					{#if !collapsed}
						<span class="text-sm font-medium truncate">{item.label}</span>
					{/if}
					{#if item.badge && !collapsed}
						<span
							class="ml-auto px-1.5 py-0.5 text-[10px] font-bold rounded-full
							       bg-tw-pink text-white leading-none min-w-[18px] text-center"
						>{item.badge}</span>
					{:else if isActive(item.href ?? '') && !collapsed}
						<span
							class="ml-auto w-1.5 h-1.5 rounded-full bg-tw-neon
							       shadow-[0_0_6px_rgba(0,245,255,0.6)]"
						></span>
					{/if}
				</a>
			{/if}
		{/each}
	</nav>

	<!-- Bottom nav items (trash, etc.) -->
	<div class="px-3 pb-2 shrink-0">
		{#each bottomNavItems as item}
			<a
				href={item.href}
				class="group flex items-center gap-3 px-3 py-2.5 rounded-xl
				       no-underline transition-all duration-200 whitespace-nowrap overflow-hidden
				       {isActive(item.href)
					? 'bg-tw-purple/20 text-tw-neon shadow-[inset_0_0_12px_rgba(0,245,255,0.08)]'
					: 'text-white/50 hover:text-white hover:bg-white/5'}"
			>
				{@render navIcon(item.icon)}
				{#if !collapsed}
					<span class="text-sm font-medium truncate">{item.label}</span>
				{/if}
				{#if isActive(item.href) && !collapsed}
					<span
						class="ml-auto w-1.5 h-1.5 rounded-full bg-tw-neon
						       shadow-[0_0_6px_rgba(0,245,255,0.6)]"
					></span>
				{/if}
			</a>
		{/each}
	</div>

	<!-- User section -->
	<div class="border-t border-white/10 px-3 py-4 shrink-0">
		<a
			href="/settings"
			class="flex items-center gap-3 px-2 py-2 mb-2 rounded-xl no-underline
			       overflow-hidden transition-colors duration-200 cursor-pointer
			       text-white/70 hover:text-white hover:bg-white/5
			       {collapsed ? 'justify-center' : ''}"
			title="Settings"
		>
			<div
				class="w-8 h-8 rounded-full bg-linear-to-br from-tw-purple to-tw-pink
				       flex items-center justify-center text-white text-xs font-bold shrink-0"
			>
				{($user?.username ?? '?')[0].toUpperCase()}
			</div>
			{#if !collapsed}
				<span class="text-sm truncate">{$user?.username}</span>
			{/if}
		</a>

		<button
			onclick={handleSignOut}
			class="w-full flex items-center gap-3 px-3 py-2 rounded-xl
			       text-white/40 hover:text-red-400 hover:bg-red-400/10
			       cursor-pointer transition-colors duration-200
			       whitespace-nowrap overflow-hidden"
		>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="1.8"
				stroke-linecap="round"
				stroke-linejoin="round"
				class="w-5 h-5 shrink-0"
			>
				<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
				<polyline points="16 17 21 12 16 7" />
				<line x1="21" y1="12" x2="9" y2="12" />
			</svg>
			{#if !collapsed}
				<span class="text-sm font-medium">Sign Out</span>
			{/if}
		</button>
	</div>

	<!-- Drag handle -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		onmousedown={startDrag}
		class="absolute top-0 right-0 w-1.5 h-full cursor-col-resize
		       hover:bg-tw-neon/30 active:bg-tw-neon/40
		       transition-colors duration-150"
	></div>
</aside>

{#if mode === 'home'}
<Modal bind:open={showNewFolder} title="New Folder">
	<form
		onsubmit={(e) => {
			e.preventDefault();
			createFolder();
		}}
		class="flex flex-col gap-4"
	>
		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Folder Name</span>
			<input
				type="text"
				bind:value={folderName}
				placeholder="My Folder"
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
				       border border-tw-purple/40 text-white
				       placeholder:text-white/30
				       focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>
		{#if folderError}
			<p class="text-sm text-red-400">{folderError}</p>
		{/if}
		<button
			type="submit"
			disabled={!folderName.trim()}
			class="rounded-lg py-2.5 font-semibold text-white
			       transition-colors duration-200
			       focus:outline-none focus:ring-2 focus:ring-tw-neon
			       {folderName.trim()
				? 'bg-tw-purple hover:bg-tw-pink cursor-pointer'
				: 'bg-white/10 text-white/30 cursor-not-allowed'}"
		>
			Create
		</button>
	</form>
</Modal>

<Modal bind:open={showUpload} title="Upload Images">
	<div class="flex flex-col gap-4">
		<label
			class="flex flex-col items-center justify-center gap-2 p-6 rounded-xl
			       border-2 border-dashed border-tw-purple/40 hover:border-tw-neon/50
			       cursor-pointer transition-colors duration-200"
		>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="1.5"
				stroke-linecap="round"
				stroke-linejoin="round"
				class="w-10 h-10 text-white/20"
			>
				<rect x="3" y="3" width="18" height="18" rx="2" />
				<circle cx="8.5" cy="8.5" r="1.5" />
				<path d="M21 15l-5-5L5 21" />
			</svg>
			<span class="text-sm text-white/30">Click to select images</span>
			<input type="file" accept="image/*" multiple onchange={onFileSelect} class="hidden" />
		</label>

		{#if uploadFiles.length > 0}
			<div class="grid grid-cols-3 gap-2 max-h-60 overflow-y-auto">
				{#each uploadFiles as file, i}
					<div class="relative group">
						<img src={URL.createObjectURL(file)} alt={file.name} class="w-full h-20 rounded-lg object-cover" />
						<button
							onclick={() => removeFile(i)}
							class="absolute top-1 right-1 w-5 h-5 rounded-full bg-black/60 text-white/80
							       text-xs flex items-center justify-center opacity-0 group-hover:opacity-100
							       transition-opacity cursor-pointer"
						>&times;</button>
						<span class="text-[10px] text-white/40 truncate block mt-0.5">{file.name}</span>
					</div>
				{/each}
			</div>
		{/if}

		{#if uploadError}
			<p class="text-sm text-red-400">{uploadError}</p>
		{/if}

		<button
			onclick={uploadImages}
			disabled={uploadFiles.length === 0 || uploading}
			class="rounded-lg py-2.5 font-semibold text-white
			       transition-colors duration-200
			       focus:outline-none focus:ring-2 focus:ring-tw-neon
			       {uploadFiles.length > 0 && !uploading
				? 'bg-tw-purple hover:bg-tw-pink cursor-pointer'
				: 'bg-white/10 text-white/30 cursor-not-allowed'}"
		>
			{uploading ? 'Uploading...' : `Upload ${uploadFiles.length > 0 ? `(${uploadFiles.length})` : ''}`}
		</button>
	</div>
</Modal>

<Modal bind:open={showShareItem} title="Share Item">
	<form
		onsubmit={(e) => { e.preventDefault(); shareItem(); }}
		class="flex flex-col gap-4"
	>
		<div class="flex flex-col gap-1.5">
			<span class="text-tw-yellow text-sm">Select an item</span>
			{#if shareableItems.length === 0}
				<p class="text-sm text-white/30">No items in this folder to share.</p>
			{:else}
				<div class="max-h-48 overflow-y-auto rounded-xl border border-tw-purple/30 bg-tw-darkblue/50 p-1.5 flex flex-col gap-0.5">
					{#each shareableItems as item}
						<button
							type="button"
							onclick={() => (shareSelectedId = item.id)}
							class="flex items-center gap-3 px-3 py-2 rounded-lg text-left
							       transition-all duration-150 cursor-pointer
							       {shareSelectedId === item.id
								? 'bg-tw-purple/20 border border-tw-neon/40 text-white'
								: 'border border-transparent text-white/60 hover:text-white hover:bg-white/5'}"
						>
							{#if item.type === 'folder'}
								<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-5 h-5 shrink-0 text-tw-yellow/60">
									<path d="M2 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z" />
								</svg>
							{:else}
								<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-5 h-5 shrink-0 text-tw-pink/60">
									<rect x="3" y="3" width="18" height="18" rx="2" />
									<circle cx="8.5" cy="8.5" r="1.5" />
									<path d="M21 15l-5-5L5 21" />
								</svg>
							{/if}
							<span class="text-sm truncate">{item.name}</span>
							{#if shareSelectedId === item.id}
								<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" class="w-4 h-4 ml-auto shrink-0 text-tw-neon">
									<polyline points="20 6 9 17 4 12" />
								</svg>
							{/if}
						</button>
					{/each}
				</div>
			{/if}
		</div>

		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Share with username</span>
			<input
				type="text"
				bind:value={shareUsername}
				placeholder="Enter username"
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
				       border border-tw-purple/40 text-white
				       placeholder:text-white/30
				       focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>

		{#if shareError}
			<p class="text-sm text-red-400">{shareError}</p>
		{/if}

		<button
			type="submit"
			disabled={!shareSelectedId || !shareUsername.trim() || sharing}
			class="rounded-lg py-2.5 font-semibold text-white
			       transition-colors duration-200
			       focus:outline-none focus:ring-2 focus:ring-tw-neon
			       {shareSelectedId && shareUsername.trim() && !sharing
				? 'bg-tw-purple hover:bg-tw-pink cursor-pointer'
				: 'bg-white/10 text-white/30 cursor-not-allowed'}"
		>
			{sharing ? 'Sharing...' : 'Share'}
		</button>
	</form>
</Modal>
{/if}
