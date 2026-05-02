<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { user } from '$lib/stores/auth';
	import { connectWebSocket, disconnectWebSocket } from '$lib/stores/websocket';
	import { fetchGroups } from '$lib/stores/groups';
	import favicon from '$lib/assets/favicon.svg';
	import { API_BASE } from '$lib/config';
	import "../app.css";
	import { fetchClient } from '$lib/api';

	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import RenameModal from '$lib/components/RenameModal.svelte';
	import ImageViewer from '$lib/components/ImageViewer.svelte';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import UploadDropzone from '$lib/components/UploadDropzone.svelte';
	import { contextMenu, renameModal, imageViewer, confirmModal, openUploadModal } from '$lib/stores/ui';

	let { children } = $props();

	const publicRoutes = ['/', '/signin', '/register'];

	onMount(async () => {
		try {
			const res = await fetchClient(`${API_BASE}/check_auth`, {
				credentials: 'include'
			});

			if (res.ok) {
				const data = await res.json();
				user.set({ 
					user_id: data.user_id, 
					username: data.username, 
					session_id: data.session_id,
					preferences: data.preferences
				});
				connectWebSocket();

				// Populate the invite badge / groups list. No-op when in group context
				// (ListGroups is user-only); auto-recovery for stale group context
				// happens reactively in op() if the backend rejects X-Group.
				fetchGroups();

				if (publicRoutes.includes(page.url.pathname)) {
					goto('/home');
				}
			} else {
				user.set(null);
				disconnectWebSocket();
				if (!publicRoutes.includes(page.url.pathname)) {
					goto('/signin');
				}
			}
		} catch {
			user.set(null);
			disconnectWebSocket();
			if (!publicRoutes.includes(page.url.pathname)) {
				goto('/signin');
			}
		}
	});

	onDestroy(() => {
		disconnectWebSocket();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}

<ContextMenu
	bind:open={$contextMenu.open}
	x={$contextMenu.x}
	y={$contextMenu.y}
	items={$contextMenu.items}
/>

<RenameModal
	bind:open={$renameModal.open}
	title={$renameModal.title}
	currentName={$renameModal.currentName}
	onSubmit={$renameModal.onSubmit}
/>

<ImageViewer
	bind:open={$imageViewer.open}
	id={$imageViewer.id}
	name={$imageViewer.name}
	url={$imageViewer.url}
	readonly={$imageViewer.readonly}
/>

<ConfirmModal
	bind:open={$confirmModal.open}
	title={$confirmModal.title}
	confirmLabel={$confirmModal.confirmLabel}
	danger={$confirmModal.danger}
	onConfirm={$confirmModal.onConfirm}
>
	<p class="text-sm text-white/70">{$confirmModal.message}</p>
</ConfirmModal>

<UploadDropzone onDrop={openUploadModal} />
