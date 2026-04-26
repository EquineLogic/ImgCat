import { writable, derived, get } from 'svelte/store';
import { op } from '$lib/api';
import { groupContext } from './groupContext';

export type GroupMemberState = 'PendingInvite' | 'Accepted';

export type GroupMember = {
	id: string;
	group_id: string;
	group_type: string;
	group_name: string;
	group_username: string;
	user_id: string;
	user_type: string;
	member_username: string | null;
	member_name: string | null;
	sender_id: string | null;
	sender_type: string | null;
	sender_username: string | null;
	perms: string[];
	state: GroupMemberState;
	created_at: string;
};

export const groupMemberships = writable<GroupMember[]>([]);

export const acceptedGroups = derived(groupMemberships, ($all) =>
	$all.filter((m) => m.state === 'Accepted')
);

export const pendingInvites = derived(groupMemberships, ($all) =>
	$all.filter((m) => m.state === 'PendingInvite')
);

/// Members of the *current* group (only meaningful while in group context).
export const currentGroupMembers = writable<GroupMember[]>([]);

export async function fetchGroups() {
	// ListGroups is a user-only op; skip when operating as a group.
	if (get(groupContext)) return;
	try {
		const r = await op<{ op: 'GroupMembers'; group_members: GroupMember[] }>({ op: 'ListGroups' });
		groupMemberships.set(r.group_members);
	} catch {
		// leave existing list as-is on failure
	}
}

/// Fetch members of the *current* group (group context required).
export async function fetchCurrentGroupMembers() {
	if (!get(groupContext)) {
		currentGroupMembers.set([]);
		return;
	}
	try {
		const r = await op<{ op: 'GroupMembers'; group_members: GroupMember[] }>({
			op: 'ListGroupMembers'
		});
		currentGroupMembers.set(r.group_members);
	} catch {
		// leave existing list as-is on failure
	}
}

export async function createGroup(username: string, name: string): Promise<string> {
	const r = await op<{ op: 'CreatedGroup'; group_id: string }>({
		op: 'CreateGroup',
		username,
		name
	});
	await fetchGroups();
	return r.group_id;
}

export async function acceptGroupInvite(group_id: string) {
	await op({ op: 'AcceptGroupInvite', group_id });
	await fetchGroups();
}

export async function denyGroupInvite(group_id: string) {
	await op({ op: 'DenyGroupInvite', group_id });
	await fetchGroups();
}

export async function inviteGroupMember(target_id: string, perms: string[]) {
	await op({ op: 'InviteGroupMember', target_id, perms });
	await fetchCurrentGroupMembers();
}

export async function removeGroupMember(user_id: string) {
	await op({ op: 'RemoveGroupMember', user_id });
	await fetchCurrentGroupMembers();
}

export async function updateGroupMemberPerms(user_id: string, perms: string[]) {
	await op({ op: 'UpdateGroupMemberPerms', user_id, perms });
	await fetchCurrentGroupMembers();
}

export type FoundUser = { user_id: string; username: string; name: string };

export async function lookupUser(username: string): Promise<FoundUser> {
	const r = await op<{ op: 'FoundUser' } & FoundUser>({ op: 'LookupUser', username });
	return { user_id: r.user_id, username: r.username, name: r.name };
}
