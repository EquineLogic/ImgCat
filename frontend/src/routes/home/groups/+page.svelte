<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import Modal from '$lib/components/Modal.svelte';
	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import {
		acceptedGroups,
		pendingInvites,
		currentGroupMembers,
		fetchGroups,
		fetchCurrentGroupMembers,
		createGroup,
		acceptGroupInvite,
		denyGroupInvite,
		inviteGroupMember,
		removeGroupMember,
		updateGroupMemberPerms,
		lookupUser,
		type FoundUser,
		type GroupMember
	} from '$lib/stores/groups';
	import { groupContext } from '$lib/stores/groupContext';
	import { resetToRoot, fetchFolders } from '$lib/stores/folders';
	import { fetchFiles } from '$lib/stores/files';
	import { user } from '$lib/stores/auth';

	let busyId = $state<string | null>(null);
	let error = $state('');

	// Create-group modal
	let showCreate = $state(false);
	let createName = $state('');
	let createUsername = $state('');
	let createError = $state('');
	let creating = $state(false);

	// Invite-member modal (only available in group context)
	let showInvite = $state(false);
	let inviteUsername = $state('');
	let invitePerms = $state('global.*');
	let inviteError = $state('');
	let inviting = $state(false);
	let inviteSuccess = $state('');
	let foundUser = $state<FoundUser | null>(null);
	let lookupPending = $state(false);
	let lookupTimer: ReturnType<typeof setTimeout> | null = null;

	function onInviteUsernameInput() {
		foundUser = null;
		inviteError = '';
		inviteSuccess = '';
		if (lookupTimer) clearTimeout(lookupTimer);
		const q = inviteUsername.trim();
		if (!q) {
			lookupPending = false;
			return;
		}
		lookupPending = true;
		lookupTimer = setTimeout(async () => {
			try {
				const r = await lookupUser(q);
				// Only commit if the input hasn't changed since the request fired.
				if (r.username === inviteUsername.trim()) foundUser = r;
			} catch {
				foundUser = null;
			} finally {
				lookupPending = false;
			}
		}, 300);
	}

	onMount(() => {
		// In personal context: refresh the user's group list / invites.
		// In group context: refresh the current group's member list.
		if ($groupContext) fetchCurrentGroupMembers();
		else fetchGroups();
	});

	async function handleAccept(group_id: string) {
		busyId = group_id;
		error = '';
		try {
			await acceptGroupInvite(group_id);
		} catch (e: any) {
			error = e?.message || 'Failed to accept invite';
		} finally {
			busyId = null;
		}
	}

	async function handleDeny(group_id: string) {
		busyId = group_id;
		error = '';
		try {
			await denyGroupInvite(group_id);
		} catch (e: any) {
			error = e?.message || 'Failed to deny invite';
		} finally {
			busyId = null;
		}
	}

	async function switchInto(group_id: string, group_username: string) {
		groupContext.set({ group_id, group_username });
		resetToRoot();
		await goto('/home');
		await Promise.all([fetchFolders(), fetchFiles()]);
	}

	async function switchToPersonal() {
		groupContext.set(null);
		resetToRoot();
		await goto('/home');
		await Promise.all([fetchFolders(), fetchFiles()]);
	}

	async function handleCreate() {
		if (!createName.trim() || !createUsername.trim()) return;
		creating = true;
		createError = '';
		try {
			await createGroup(createUsername.trim(), createName.trim());
			createName = '';
			createUsername = '';
			showCreate = false;
		} catch (e: any) {
			createError = e?.message || 'Failed to create group';
		} finally {
			creating = false;
		}
	}

	async function handleInvite() {
		const q = inviteUsername.trim();
		if (!q) return;
		inviting = true;
		inviteError = '';
		inviteSuccess = '';
		try {
			// Use cached lookup if it matches, otherwise re-fetch.
			const target = foundUser && foundUser.username === q ? foundUser : await lookupUser(q);
			const perms = invitePerms
				.split(',')
				.map((p) => p.trim())
				.filter(Boolean);
			await inviteGroupMember(target.user_id, perms);
			inviteSuccess = `Invite sent to @${target.username}`;
			inviteUsername = '';
			foundUser = null;
		} catch (e: any) {
			inviteError = e?.message || 'Failed to send invite';
		} finally {
			inviting = false;
		}
	}

	const inGroup = $derived($groupContext !== null);

	// Find my own row in the current group's member list to inspect my perms.
	const myMembership = $derived<GroupMember | undefined>(
		$user
			? $currentGroupMembers.find((m) => m.user_id === $user!.user_id)
			: undefined
	);
	const canManageMembers = $derived(myMembership?.perms.includes('global.*') ?? false);

	// Member context menu state
	let memberMenuOpen = $state(false);
	let memberMenuX = $state(0);
	let memberMenuY = $state(0);
	let memberMenuTarget = $state<GroupMember | null>(null);

	function openMemberMenu(e: MouseEvent, m: GroupMember) {
		if (!canManageMembers) return;
		// Don't surface a "remove yourself" entry — leaving the group is its own flow.
		if (m.user_id === $user?.user_id) return;
		e.preventDefault();
		memberMenuTarget = m;
		memberMenuX = e.clientX;
		memberMenuY = e.clientY;
		memberMenuOpen = true;
	}

	const memberMenuItems = $derived(
		memberMenuTarget
			? [
					{
						label: 'Change permissions',
						icon: 'rename',
						action: () => {
							if (!memberMenuTarget) return;
							openPermsModal(memberMenuTarget);
						}
					},
					{
						label: 'Remove from group',
						icon: 'delete',
						danger: true,
						action: async () => {
							if (!memberMenuTarget) return;
							try {
								await removeGroupMember(memberMenuTarget.user_id);
							} catch (e: any) {
								error = e?.message || 'Failed to remove member';
							}
						}
					}
				]
			: []
	);

	// ── Permission picker ────────────────────────────────────────────────
	type PresetKey = 'full' | 'editor' | 'viewer' | 'custom';
	type Preset = {
		key: PresetKey;
		label: string;
		description: string;
		perms: string[]; // empty for "custom"
	};

	const PRESETS: Preset[] = [
		{
			key: 'full',
			label: 'Full access',
			description: 'Can do anything, including managing members and their perms.',
			perms: ['global.*']
		},
		{
			key: 'editor',
			label: 'Editor',
			description: 'Manage all files and folders. Cannot manage members.',
			perms: ['fs.*']
		},
		{
			key: 'viewer',
			label: 'Viewer',
			description: 'Read-only — can browse files and folders but cannot change anything.',
			perms: ['fs.list_folder', 'fs.list_files', 'fs.list_trash']
		},
		{
			key: 'custom',
			label: 'Custom',
			description: 'Specify exact perms. Comma-separated kittycat patterns.',
			perms: []
		}
	];

	function detectPreset(perms: string[]): PresetKey {
		const sorted = [...perms].sort().join(',');
		for (const p of PRESETS) {
			if (p.key === 'custom') continue;
			if ([...p.perms].sort().join(',') === sorted) return p.key;
		}
		return 'custom';
	}

	let showPermsModal = $state(false);
	let permsTarget = $state<GroupMember | null>(null);
	let selectedPreset = $state<PresetKey>('viewer');
	let customPerms = $state('');
	let permsSaving = $state(false);
	let permsError = $state('');

	function openPermsModal(m: GroupMember) {
		permsTarget = m;
		selectedPreset = detectPreset(m.perms);
		customPerms = m.perms.join(', ');
		permsError = '';
		showPermsModal = true;
	}

	function permsForSelection(): string[] {
		if (selectedPreset === 'custom') {
			return customPerms
				.split(',')
				.map((p) => p.trim())
				.filter(Boolean);
		}
		return PRESETS.find((p) => p.key === selectedPreset)?.perms ?? [];
	}

	async function handleSavePerms() {
		if (!permsTarget) return;
		const perms = permsForSelection();
		if (perms.length === 0) {
			permsError = 'Pick at least one permission';
			return;
		}
		permsSaving = true;
		permsError = '';
		try {
			await updateGroupMemberPerms(permsTarget.user_id, perms);
			showPermsModal = false;
			permsTarget = null;
		} catch (e: any) {
			permsError = e?.message || 'Failed to update permissions';
		} finally {
			permsSaving = false;
		}
	}
</script>

<div class="p-8 max-w-3xl">
	<div class="flex items-center justify-between mb-6">
		<h1 class="text-xl font-semibold text-white">Groups</h1>
		<div class="flex items-center gap-2">
			{#if inGroup}
				<button
					onclick={() => (showInvite = true)}
					class="px-3 py-1.5 rounded-lg text-sm font-medium
					       bg-tw-purple hover:bg-tw-pink text-white
					       cursor-pointer transition-colors duration-150"
				>
					Invite Member
				</button>
			{:else}
				<button
					onclick={() => (showCreate = true)}
					class="px-3 py-1.5 rounded-lg text-sm font-medium
					       bg-tw-purple hover:bg-tw-pink text-white
					       cursor-pointer transition-colors duration-150"
				>
					Create Group
				</button>
			{/if}
		</div>
	</div>

	<!-- Current context indicator -->
	<section class="mb-6 rounded-2xl border border-white/10 bg-tw-darkblue/40 p-4">
		<div class="flex items-center justify-between gap-3">
			<div class="flex items-center gap-3">
				<div
					class="w-10 h-10 rounded-full flex items-center justify-center text-white text-sm font-bold shrink-0
					       {inGroup
						? 'bg-linear-to-br from-tw-yellow to-tw-pink'
						: 'bg-linear-to-br from-tw-purple to-tw-pink'}"
				>
					{($groupContext?.group_username ?? 'P')[0].toUpperCase()}
				</div>
				<div class="flex flex-col">
					<span class="text-xs text-white/40 uppercase tracking-wider">
						{inGroup ? 'Acting as group' : 'Acting as'}
					</span>
					<span class="text-sm text-white">
						{$groupContext?.group_username ?? 'Personal'}
					</span>
				</div>
			</div>
			{#if inGroup}
				<button
					onclick={switchToPersonal}
					class="text-xs px-2.5 py-1 rounded-md text-tw-neon/80 hover:text-tw-neon
					       border border-tw-neon/30 hover:border-tw-neon/60 hover:bg-tw-neon/10
					       cursor-pointer transition-colors duration-150"
				>
					Switch to personal
				</button>
			{/if}
		</div>
	</section>

	{#if error}
		<div class="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-sm text-red-300">
			{error}
		</div>
	{/if}

	{#if inGroup}
		<section>
			<h2 class="text-sm font-semibold text-white/40 uppercase tracking-wider mb-3">
				Members ({$currentGroupMembers.length})
			</h2>
			{#if $currentGroupMembers.length === 0}
				<p class="text-sm text-white/30">No members yet.</p>
			{:else}
				<div class="flex flex-col gap-2">
					{#each $currentGroupMembers as m (m.id)}
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<div
							ondblclick={(e) => openMemberMenu(e, m)}
							oncontextmenu={(e) => openMemberMenu(e, m)}
							class="flex items-center justify-between gap-3 px-4 py-3 rounded-xl
							       bg-white/5 border border-white/10
							       {canManageMembers && m.user_id !== $user?.user_id
								? 'cursor-context-menu hover:bg-white/10'
								: ''}"
						>
							<div class="flex items-center gap-3 min-w-0">
								<div
									class="w-9 h-9 rounded-full bg-linear-to-br from-tw-purple to-tw-pink
									       flex items-center justify-center text-white text-xs font-bold shrink-0"
								>
									{(m.member_username ?? '?')[0].toUpperCase()}
								</div>
								<div class="flex flex-col min-w-0">
									<span class="text-sm text-white truncate">
										{m.member_name ?? m.member_username ?? m.user_id}
									</span>
									<span class="text-xs text-white/40 truncate">
										{#if m.member_username}@{m.member_username}{/if}
										{#if m.sender_username && m.sender_id !== m.user_id}
											&middot; invited by @{m.sender_username}
										{:else if m.sender_id === m.user_id}
											&middot; owner
										{/if}
									</span>
								</div>
							</div>
							<div class="flex items-center gap-2 shrink-0">
								{#if m.state === 'PendingInvite'}
									<span
										class="px-2 py-0.5 rounded-md text-[10px] font-semibold uppercase tracking-wider
										       bg-tw-yellow/15 text-tw-yellow border border-tw-yellow/30"
									>
										Pending
									</span>
								{:else}
									<span
										class="px-2 py-0.5 rounded-md text-[10px] font-semibold uppercase tracking-wider
										       bg-tw-neon/15 text-tw-neon border border-tw-neon/30"
									>
										Accepted
									</span>
								{/if}
								<span class="text-[10px] text-white/30 font-mono truncate max-w-40">
									{m.perms.join(', ')}
								</span>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</section>
		<p class="text-xs text-white/30 mt-4">
			Switch back to personal to manage your invites or join other groups.
		</p>
	{:else}
		<!-- Pending invites -->
		{#if $pendingInvites.length > 0}
			<section class="mb-8">
				<h2 class="text-sm font-semibold text-tw-yellow uppercase tracking-wider mb-3">
					Pending Invites ({$pendingInvites.length})
				</h2>
				<div class="flex flex-col gap-2">
					{#each $pendingInvites as inv (inv.id)}
						<div
							class="flex items-center justify-between gap-3 px-4 py-3 rounded-xl
							       bg-white/5 border border-white/10"
						>
							<div class="flex flex-col min-w-0">
								<span class="text-sm text-white truncate">{inv.group_name}</span>
								<span class="text-xs text-white/40 truncate">
									@{inv.group_username}
									{#if inv.sender_username}
										&middot; invited by @{inv.sender_username}
									{/if}
								</span>
							</div>
							<div class="flex gap-2 shrink-0">
								<button
									onclick={() => handleAccept(inv.group_id)}
									disabled={busyId === inv.group_id}
									class="px-3 py-1.5 rounded-lg text-xs font-semibold text-white
									       bg-tw-purple hover:bg-tw-pink cursor-pointer transition-colors
									       disabled:opacity-50 disabled:cursor-not-allowed"
								>
									Accept
								</button>
								<button
									onclick={() => handleDeny(inv.group_id)}
									disabled={busyId === inv.group_id}
									class="px-3 py-1.5 rounded-lg text-xs font-semibold text-white/60
									       bg-white/5 hover:bg-white/10 cursor-pointer transition-colors
									       disabled:opacity-50 disabled:cursor-not-allowed"
								>
									Deny
								</button>
							</div>
						</div>
					{/each}
				</div>
			</section>
		{/if}

		<!-- Accepted groups -->
		<section>
			<h2 class="text-sm font-semibold text-white/40 uppercase tracking-wider mb-3">
				Your Groups
			</h2>
			{#if $acceptedGroups.length === 0}
				<p class="text-sm text-white/30">
					You aren't a member of any groups yet. Create one above.
				</p>
			{:else}
				<div class="flex flex-col gap-2">
					{#each $acceptedGroups as g (g.id)}
						<div
							class="flex items-center justify-between gap-3 px-4 py-3 rounded-xl
							       bg-white/5 border border-white/10"
						>
							<div class="flex items-center gap-3 min-w-0">
								<div
									class="w-9 h-9 rounded-full bg-linear-to-br from-tw-yellow to-tw-pink
									       flex items-center justify-center text-white text-xs font-bold shrink-0"
								>
									{g.group_username[0].toUpperCase()}
								</div>
								<div class="flex flex-col min-w-0">
									<span class="text-sm text-white truncate">{g.group_name}</span>
									<span class="text-xs text-white/40 truncate">@{g.group_username}</span>
								</div>
							</div>
							<button
								onclick={() => switchInto(g.group_id, g.group_username)}
								class="px-3 py-1.5 rounded-lg text-xs font-semibold text-tw-neon/80
								       hover:text-tw-neon border border-tw-neon/30 hover:border-tw-neon/60
								       hover:bg-tw-neon/10 cursor-pointer transition-colors shrink-0"
							>
								Switch in
							</button>
						</div>
					{/each}
				</div>
			{/if}
		</section>
	{/if}
</div>

<Modal bind:open={showCreate} title="Create Group">
	<form
		onsubmit={(e) => {
			e.preventDefault();
			handleCreate();
		}}
		class="flex flex-col gap-4"
	>
		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Display name</span>
			<input
				type="text"
				bind:value={createName}
				placeholder="My Cool Group"
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
				       border border-tw-purple/40 text-white
				       placeholder:text-white/30
				       focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>
		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Username (handle)</span>
			<input
				type="text"
				bind:value={createUsername}
				placeholder="cool_group"
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
				       border border-tw-purple/40 text-white
				       placeholder:text-white/30
				       focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>
		{#if createError}
			<p class="text-sm text-red-400">{createError}</p>
		{/if}
		<button
			type="submit"
			disabled={!createName.trim() || !createUsername.trim() || creating}
			class="rounded-lg py-2.5 font-semibold text-white
			       transition-colors duration-200
			       focus:outline-none focus:ring-2 focus:ring-tw-neon
			       {createName.trim() && createUsername.trim() && !creating
				? 'bg-tw-purple hover:bg-tw-pink cursor-pointer'
				: 'bg-white/10 text-white/30 cursor-not-allowed'}"
		>
			{creating ? 'Creating...' : 'Create'}
		</button>
	</form>
</Modal>

<Modal bind:open={showInvite} title="Invite Member">
	<form
		onsubmit={(e) => {
			e.preventDefault();
			handleInvite();
		}}
		class="flex flex-col gap-4"
	>
		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Username</span>
			<input
				type="text"
				bind:value={inviteUsername}
				oninput={onInviteUsernameInput}
				placeholder="twilight_sparkle"
				autocomplete="off"
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
				       border border-tw-purple/40 text-white
				       placeholder:text-white/30
				       focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
			{#if inviteUsername.trim() && lookupPending}
				<span class="text-xs text-white/40">Looking up...</span>
			{:else if foundUser}
				<span class="text-xs text-green-400">
					Found: {foundUser.name} (@{foundUser.username})
				</span>
			{:else if inviteUsername.trim() && !lookupPending}
				<span class="text-xs text-red-400/70">No user with that username</span>
			{/if}
		</label>
		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Permissions (comma separated)</span>
			<input
				type="text"
				bind:value={invitePerms}
				placeholder="global.*"
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
				       border border-tw-purple/40 text-white text-sm font-mono
				       placeholder:text-white/30
				       focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
			<span class="text-xs text-white/30">
				Use <code>global.*</code> for full access, or e.g.
				<code>fs.list_files, fs.list_folder</code> for read-only.
			</span>
		</label>
		{#if inviteError}
			<p class="text-sm text-red-400">{inviteError}</p>
		{/if}
		{#if inviteSuccess}
			<p class="text-sm text-green-400">{inviteSuccess}</p>
		{/if}
		<button
			type="submit"
			disabled={!foundUser || inviting}
			class="rounded-lg py-2.5 font-semibold text-white
			       transition-colors duration-200
			       focus:outline-none focus:ring-2 focus:ring-tw-neon
			       {foundUser && !inviting
				? 'bg-tw-purple hover:bg-tw-pink cursor-pointer'
				: 'bg-white/10 text-white/30 cursor-not-allowed'}"
		>
			{inviting ? 'Sending...' : 'Send Invite'}
		</button>
	</form>
</Modal>

<ContextMenu bind:open={memberMenuOpen} x={memberMenuX} y={memberMenuY} items={memberMenuItems} />

<Modal
	bind:open={showPermsModal}
	title={permsTarget ? `Permissions for ${permsTarget.member_name ?? '@' + (permsTarget.member_username ?? '?')}` : 'Permissions'}
>
	<form
		onsubmit={(e) => {
			e.preventDefault();
			handleSavePerms();
		}}
		class="flex flex-col gap-4"
	>
		<div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
			{#each PRESETS as preset (preset.key)}
				<button
					type="button"
					onclick={() => (selectedPreset = preset.key)}
					class="text-left p-3 rounded-xl border transition-all duration-150 cursor-pointer
					       {selectedPreset === preset.key
						? 'border-tw-neon/60 bg-tw-neon/10 shadow-[inset_0_0_12px_rgba(0,245,255,0.08)]'
						: 'border-white/10 bg-white/2 hover:border-white/20 hover:bg-white/5'}"
				>
					<div class="flex items-center gap-2 mb-1">
						<span class="text-sm font-semibold text-white">{preset.label}</span>
						{#if selectedPreset === preset.key}
							<svg
								xmlns="http://www.w3.org/2000/svg"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2.5"
								class="w-3.5 h-3.5 ml-auto text-tw-neon shrink-0"
							>
								<polyline points="20 6 9 17 4 12" />
							</svg>
						{/if}
					</div>
					<p class="text-xs text-white/50 leading-snug">{preset.description}</p>
					{#if preset.key !== 'custom'}
						<p class="mt-2 text-[10px] font-mono text-white/30 truncate">
							{preset.perms.join(', ')}
						</p>
					{/if}
				</button>
			{/each}
		</div>

		{#if selectedPreset === 'custom'}
			<label class="flex flex-col gap-1">
				<span class="text-tw-yellow text-sm">Permissions (comma separated)</span>
				<input
					type="text"
					bind:value={customPerms}
					placeholder="fs.list_files, fs.upload_file"
					class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
					       border border-tw-purple/40 text-white text-sm font-mono
					       placeholder:text-white/30
					       focus:outline-none focus:ring-2 focus:ring-tw-neon"
				/>
				<span class="text-xs text-white/30">
					Use kittycat patterns like <code>fs.*</code>, <code>group_members.invite</code>, or
					<code>global.*</code> for everything.
				</span>
			</label>
		{/if}

		{#if permsTarget}
			<div class="text-xs text-white/40">
				Current: <span class="font-mono text-white/60">{permsTarget.perms.join(', ') || '(none)'}</span>
			</div>
		{/if}

		{#if permsError}
			<p class="text-sm text-red-400">{permsError}</p>
		{/if}

		<div class="flex gap-2 justify-end">
			<button
				type="button"
				onclick={() => (showPermsModal = false)}
				class="px-4 py-2 rounded-lg text-sm text-white/60 hover:text-white hover:bg-white/10
				       cursor-pointer transition-colors duration-150"
			>
				Cancel
			</button>
			<button
				type="submit"
				disabled={permsSaving}
				class="px-4 py-2 rounded-lg text-sm font-semibold text-white
				       transition-colors duration-200
				       focus:outline-none focus:ring-2 focus:ring-tw-neon
				       {permsSaving
					? 'bg-white/10 text-white/30 cursor-not-allowed'
					: 'bg-tw-purple hover:bg-tw-pink cursor-pointer'}"
			>
				{permsSaving ? 'Saving...' : 'Save'}
			</button>
		</div>
	</form>
</Modal>
