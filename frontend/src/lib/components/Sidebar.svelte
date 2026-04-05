<script lang="ts">
	import { user } from '$lib/stores/auth';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import Modal from './Modal.svelte';
	import { fetchFolders, currentFolderId } from '$lib/stores/folders';
	import { fetchFiles } from '$lib/stores/files';

	const MIN_W = 72;
	const MAX_W = 240;
	const COLLAPSE_THRESHOLD = 110;

	let sidebarWidth = $state(220);
	let dragging = $state(false);
	let showNewFolder = $state(false);
	let folderName = $state('');
	let folderError = $state('');

	let showUpload = $state(false);
	let uploadFile: File | null = $state(null);
	let uploadPreview: string | null = $state(null);
	let uploadName = $state('');
	let uploadError = $state('');
	let uploading = $state(false);

	const collapsed = $derived(sidebarWidth < COLLAPSE_THRESHOLD);

	const navItems = [
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
		}
	];

	const bottomNavItems = [
		{
			label: 'Trash',
			href: '/home/trash',
			icon: 'trash'
		}
	];

	async function createFolder() {
		if (!folderName.trim()) return;
		const res = await fetch('http://localhost:3000/create_folder', {
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
		const file = input.files?.[0] ?? null;
		uploadFile = file;
		if (file) {
			uploadPreview = URL.createObjectURL(file);
			// Strip extension for a cleaner default name (e.g. "cat.png" -> "cat")
			uploadName = file.name.replace(/\.[^.]+$/, '');
		} else {
			uploadPreview = null;
			uploadName = '';
		}
	}

	async function uploadImage() {
		if (!uploadFile) return;
		uploading = true;
		uploadError = '';
		const form = new FormData();
		form.append('file', uploadFile);
		form.append('name', uploadName.trim() || uploadFile.name);
		if ($currentFolderId) form.append('parent_id', $currentFolderId);
		try {
			const res = await fetch('http://localhost:3000/upload_file', {
				method: 'POST',
				credentials: 'include',
				body: form
			});
			if (!res.ok) {
				uploadError = await res.text();
				return;
			}
			uploadFile = null;
			uploadPreview = null;
			uploadName = '';
			uploadError = '';
			showUpload = false;
			await fetchFiles();
		} catch (e) {
			uploadError = 'Upload failed';
		} finally {
			uploading = false;
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
		await fetch('http://localhost:3000/signout', {
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
					{#if isActive(item.href ?? '') && !collapsed}
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
		{#if !collapsed}
			<div class="flex items-center gap-3 px-2 mb-3 overflow-hidden">
				<div
					class="w-8 h-8 rounded-full bg-linear-to-br from-tw-purple to-tw-pink
					       flex items-center justify-center text-white text-xs font-bold shrink-0"
				>
					{($user?.username ?? '?')[0].toUpperCase()}
				</div>
				<span class="text-sm text-white/70 truncate">{$user?.username}</span>
			</div>
		{:else}
			<div class="flex justify-center mb-3">
				<div
					class="w-8 h-8 rounded-full bg-linear-to-br from-tw-purple to-tw-pink
					       flex items-center justify-center text-white text-xs font-bold"
				>
					{($user?.username ?? '?')[0].toUpperCase()}
				</div>
			</div>
		{/if}

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

<Modal bind:open={showUpload} title="Upload Image">
	<div class="flex flex-col gap-4">
		<label
			class="flex flex-col items-center justify-center gap-2 p-6 rounded-xl
			       border-2 border-dashed border-tw-purple/40 hover:border-tw-neon/50
			       cursor-pointer transition-colors duration-200"
		>
			{#if uploadPreview}
				<img src={uploadPreview} alt="Preview" class="max-h-48 rounded-lg object-contain" />
				<span class="text-xs text-white/40 mt-1">{uploadFile?.name}</span>
			{:else}
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
				<span class="text-sm text-white/30">Click to select an image</span>
			{/if}
			<input type="file" accept="image/*" onchange={onFileSelect} class="hidden" />
		</label>

		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Name</span>
			<input
				type="text"
				bind:value={uploadName}
				placeholder="Image name"
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
				       border border-tw-purple/40 text-white
				       placeholder:text-white/30
				       focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>

		{#if uploadError}
			<p class="text-sm text-red-400">{uploadError}</p>
		{/if}

		<button
			onclick={uploadImage}
			disabled={!uploadFile || uploading}
			class="rounded-lg py-2.5 font-semibold text-white
			       transition-colors duration-200
			       focus:outline-none focus:ring-2 focus:ring-tw-neon
			       {uploadFile && !uploading
				? 'bg-tw-purple hover:bg-tw-pink cursor-pointer'
				: 'bg-white/10 text-white/30 cursor-not-allowed'}"
		>
			{uploading ? 'Uploading...' : 'Upload'}
		</button>
	</div>
</Modal>
