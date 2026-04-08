<script lang="ts">
	import { onMount } from 'svelte';
	import FolderCard from '$lib/components/FolderCard.svelte';
	import ImageCard from '$lib/components/ImageCard.svelte';
	import ImageViewer from '$lib/components/ImageViewer.svelte';
	import FileGrid from '$lib/components/FileGrid.svelte';
	import {
		pendingRequests,
		sharedWithMe,
		fetchPendingRequests,
		fetchSharedWithMe,
		acceptShareRequest,
		declineShareRequest,
		copySharedFile,
		fetchSharedFolder,
		fetchSharedFiles,
		type PermissionEntry,
		type Folder,
		type FileEntry
	} from '$lib/stores/sharing';

	let loading = $state(true);

	// Browsing state for shared folders
	let browsingPermission: PermissionEntry | null = $state(null);
	let browseFolders: Folder[] = $state([]);
	let browseFiles: FileEntry[] = $state([]);
	let browseStack: { id: string | null; name: string }[] = $state([]);
	let browseLoading = $state(false);

	onMount(async () => {
		await Promise.all([fetchPendingRequests(), fetchSharedWithMe()]);
		loading = false;
	});

	async function handleAccept(id: string) {
		try {
			await acceptShareRequest(id);
			await Promise.all([fetchPendingRequests(), fetchSharedWithMe()]);
		} catch {}
	}

	async function handleDecline(id: string) {
		try {
			await declineShareRequest(id);
			await fetchPendingRequests();
		} catch {}
	}

	async function openSharedFolder(perm: PermissionEntry) {
		browsingPermission = perm;
		browseStack = [{ id: null, name: perm.entry_name }];
		await loadSharedContent(perm.filesystem_id, null);
	}

	async function loadSharedContent(permFsId: string, parentId: string | null) {
		browseLoading = true;
		try {
			const [folders, files] = await Promise.all([
				fetchSharedFolder(permFsId, parentId),
				fetchSharedFiles(permFsId, parentId)
			]);
			browseFolders = folders;
			browseFiles = files;
		} catch {
			browseFolders = [];
			browseFiles = [];
		}
		browseLoading = false;
	}

	async function navigateIntoSharedFolder(folderId: string, folderName: string) {
		if (!browsingPermission) return;
		browseStack = [...browseStack, { id: folderId, name: folderName }];
		await loadSharedContent(browsingPermission.filesystem_id, folderId);
	}

	async function navigateToBreadcrumb(index: number) {
		if (!browsingPermission) return;
		browseStack = browseStack.slice(0, index + 1);
		const target = browseStack[browseStack.length - 1];
		await loadSharedContent(browsingPermission.filesystem_id, target.id);
	}

	function exitBrowse() {
		browsingPermission = null;
		browseFolders = [];
		browseFiles = [];
		browseStack = [];
	}

	let copyStatus: Record<string, string> = $state({});

	async function handleCopy(fsId: string) {
		copyStatus[fsId] = 'Copying...';
		try {
			await copySharedFile(fsId);
			copyStatus[fsId] = 'Copied!';
			setTimeout(() => { copyStatus[fsId] = ''; }, 2000);
		} catch (e: any) {
			copyStatus[fsId] = 'Failed';
			setTimeout(() => { copyStatus[fsId] = ''; }, 2000);
		}
	}
</script>

<div class="p-8">
	{#if browsingPermission}
		<!-- Browsing inside a shared folder -->
		<div class="flex items-center gap-3 mb-6">
			<button
				onclick={exitBrowse}
				class="text-white/40 hover:text-tw-neon cursor-pointer transition-colors text-sm"
			>
				Shared with Me
			</button>
			{#each browseStack as crumb, i}
				<span class="text-white/20">/</span>
				{#if i < browseStack.length - 1}
					<button
						onclick={() => navigateToBreadcrumb(i)}
						class="text-white/40 hover:text-tw-neon cursor-pointer transition-colors text-sm"
					>
						{crumb.name}
					</button>
				{:else}
					<span class="text-white text-sm">{crumb.name}</span>
				{/if}
			{/each}
		</div>

		<FileGrid
			folders={browseFolders}
			files={browseFiles}
			loading={browseLoading}
			folderItem={sharedFolderItem}
			fileItem={sharedFileItem}
		/>
	{:else}
		<!-- Main shared view -->
		<h1 class="text-xl font-bold text-white mb-6">Shared with Me</h1>

		{#if loading}
			<div class="flex justify-center mt-20">
				<div class="w-6 h-6 border-2 border-tw-neon/30 border-t-tw-neon rounded-full animate-spin"></div>
			</div>
		{:else}
			<!-- Pending requests -->
			{#if $pendingRequests.length > 0}
				<div class="mb-8">
					<h2 class="text-sm font-semibold text-tw-yellow uppercase tracking-wider mb-3">
						Pending Requests ({$pendingRequests.length})
					</h2>
					<div class="flex flex-col gap-2">
						{#each $pendingRequests as req}
							<div class="flex items-center justify-between px-4 py-3 rounded-xl bg-white/5 border border-white/10">
								<div class="flex items-center gap-3">
									{#if req.url}
										<img src={req.url} alt={req.entry_name} class="w-10 h-10 rounded-lg object-cover" />
									{:else}
										<div class="w-10 h-10 rounded-lg bg-tw-yellow/10 flex items-center justify-center">
											<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-5 h-5 text-tw-yellow/60">
												<path d="M2 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z" />
											</svg>
										</div>
									{/if}
									<div>
										<p class="text-sm text-white">{req.entry_name}</p>
										<p class="text-xs text-white/30">from {req.sender_username}</p>
									</div>
								</div>
								<div class="flex gap-2">
									<button
										onclick={() => handleAccept(req.id)}
										class="px-3 py-1.5 rounded-lg text-xs font-semibold
										       bg-tw-purple hover:bg-tw-pink text-white cursor-pointer transition-colors"
									>
										Accept
									</button>
									<button
										onclick={() => handleDecline(req.id)}
										class="px-3 py-1.5 rounded-lg text-xs font-semibold
										       bg-white/5 hover:bg-white/10 text-white/60 cursor-pointer transition-colors"
									>
										Decline
									</button>
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Accepted shares -->
			{#if $sharedWithMe.length > 0}
				<h2 class="text-sm font-semibold text-white/40 uppercase tracking-wider mb-3">Shared Items</h2>

				{@const sharedFolders = $sharedWithMe.filter(i => i.entry_type === 'folder')}
				{@const sharedFiles = $sharedWithMe.filter(i => i.entry_type === 'file_link')}

				{#if sharedFolders.length > 0}
					<div class="flex flex-wrap gap-2 mb-6">
						{#each sharedFolders as item}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<!-- svelte-ignore a11y_click_events_have_key_events -->
							<div onclick={() => openSharedFolder(item)}>
								<FolderCard name={item.entry_name} id={item.filesystem_id} readonly />
							</div>
						{/each}
					</div>
				{/if}

				{#if sharedFiles.length > 0}
					<div class="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-4">
						{#each sharedFiles as item}
							<div class="relative group">
								<ImageCard name={item.entry_name} id={item.filesystem_id} url={item.url ?? ''} readonly />
								<button
									onclick={(e) => { e.stopPropagation(); handleCopy(item.filesystem_id); }}
									class="absolute top-2 right-2 z-10 px-2 py-1 rounded-lg text-[10px] font-semibold
									       bg-tw-purple/80 hover:bg-tw-pink text-white
									       opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
								>
									{copyStatus[item.filesystem_id] || 'Copy to Library'}
								</button>
							</div>
						{/each}
					</div>
				{/if}
			{:else if $pendingRequests.length === 0}
				<div class="flex flex-col items-center gap-4 mt-20 text-center">
					<div class="w-16 h-16 rounded-2xl bg-white/5 border border-white/10 flex items-center justify-center">
						<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-8 h-8 text-white/20">
							<circle cx="18" cy="5" r="3" />
							<circle cx="6" cy="12" r="3" />
							<circle cx="18" cy="19" r="3" />
							<line x1="8.59" y1="13.51" x2="15.42" y2="17.49" />
							<line x1="15.41" y1="6.51" x2="8.59" y2="10.49" />
						</svg>
					</div>
					<p class="text-white/30 text-sm">Nothing shared with you yet.</p>
				</div>
			{/if}
		{/if}
	{/if}
</div>

{#snippet sharedFolderItem(folder: Folder)}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div onclick={() => navigateIntoSharedFolder(folder.id, folder.name)}>
		<FolderCard name={folder.name} id={folder.id} readonly />
	</div>
{/snippet}

{#snippet sharedFileItem(file: FileEntry)}
	<div class="relative group">
		<ImageCard name={file.name} id={file.id} url={file.url} readonly />
		<button
			onclick={(e) => { e.stopPropagation(); handleCopy(file.id); }}
			class="absolute top-2 right-2 z-10 px-2 py-1 rounded-lg text-[10px] font-semibold
			       bg-tw-purple/80 hover:bg-tw-pink text-white
			       opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
		>
			{copyStatus[file.id] || 'Copy to Library'}
		</button>
	</div>
{/snippet}
