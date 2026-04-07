<script lang="ts">
	import Modal from './Modal.svelte';
	import { sendShareRequest, fetchMyGrants, revokePermission, type PermissionEntry } from '$lib/stores/sharing';

	let {
		open = $bindable(false),
		filesystemId,
		entryName
	} = $props<{
		open: boolean;
		filesystemId: string;
		entryName: string;
	}>();

	let username = $state('');
	let error = $state('');
	let submitting = $state(false);
	let grants: PermissionEntry[] = $state([]);
	let loadingGrants = $state(false);

	$effect(() => {
		if (open) {
			username = '';
			error = '';
			loadGrants();
		}
	});

	async function loadGrants() {
		loadingGrants = true;
		try {
			const all = await fetchMyGrants();
			grants = all.filter((g) => g.filesystem_id === filesystemId);
		} catch {
			grants = [];
		}
		loadingGrants = false;
	}

	async function handleSubmit() {
		const trimmed = username.trim();
		if (!trimmed) return;
		submitting = true;
		error = '';
		try {
			await sendShareRequest(filesystemId, trimmed);
			username = '';
			await loadGrants();
		} catch (e: any) {
			error = e.message || 'Failed to share';
		}
		submitting = false;
	}

	async function handleRevoke(id: string) {
		try {
			await revokePermission(id);
			grants = grants.filter((g) => g.id !== id);
		} catch (e: any) {
			error = e.message || 'Failed to revoke';
		}
	}
</script>

<Modal bind:open title="Share: {entryName}">
	<form
		onsubmit={(e) => {
			e.preventDefault();
			handleSubmit();
		}}
		class="flex flex-col gap-4"
	>
		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Share with username</span>
			<div class="flex gap-2">
				<input
					type="text"
					bind:value={username}
					placeholder="Enter username"
					class="flex-1 rounded-lg px-4 py-2.5 bg-tw-darkblue/80
					       border border-tw-purple/40 text-white
					       placeholder:text-white/30
					       focus:outline-none focus:ring-2 focus:ring-tw-neon"
				/>
				<button
					type="submit"
					disabled={!username.trim() || submitting}
					class="px-4 py-2.5 rounded-lg font-semibold text-white
					       transition-colors duration-200
					       focus:outline-none focus:ring-2 focus:ring-tw-neon
					       {username.trim() && !submitting
						? 'bg-tw-purple hover:bg-tw-pink cursor-pointer'
						: 'bg-white/10 text-white/30 cursor-not-allowed'}"
				>
					{submitting ? '...' : 'Share'}
				</button>
			</div>
		</label>

		{#if error}
			<p class="text-sm text-red-400">{error}</p>
		{/if}
	</form>

	{#if grants.length > 0}
		<div class="mt-4 flex flex-col gap-2">
			<span class="text-xs text-white/40 uppercase tracking-wider">Shared with</span>
			{#each grants as grant}
				<div class="flex items-center justify-between px-3 py-2 rounded-lg bg-white/5">
					<div class="flex flex-col">
						<span class="text-sm text-white">{grant.grantee_username}</span>
						<span class="text-xs text-white/30">{grant.access_level}</span>
					</div>
					<button
						onclick={() => handleRevoke(grant.id)}
						class="text-xs text-red-400 hover:text-red-300 cursor-pointer transition-colors"
					>
						Revoke
					</button>
				</div>
			{/each}
		</div>
	{/if}
</Modal>
