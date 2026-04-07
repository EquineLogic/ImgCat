import { writable } from 'svelte/store';

export type ShareRequest = {
	id: string;
	filesystem_id: string;
	entry_name: string;
	entry_type: string;
	sender_username: string;
	recipient_username: string;
	access_level: string;
	created_at: string;
	url: string | null;
};

export type PermissionEntry = {
	id: string;
	filesystem_id: string;
	entry_name: string;
	entry_type: string;
	grantee_username: string;
	granted_by: string | null;
	access_level: string;
	created_at: string;
	url: string | null;
};

export type Folder = { id: string; name: string };
export type FileEntry = { id: string; name: string; mime_type: string; size_bytes: number; url: string };

export const pendingRequests = writable<ShareRequest[]>([]);
export const sharedWithMe = writable<PermissionEntry[]>([]);

export async function fetchPendingRequests() {
	const res = await fetch('http://localhost:3000/pending_requests', { credentials: 'include' });
	if (res.ok) pendingRequests.set(await res.json());
}

export async function fetchSharedWithMe() {
	const res = await fetch('http://localhost:3000/shared_with_me', { credentials: 'include' });
	if (res.ok) sharedWithMe.set(await res.json());
}

export async function sendShareRequest(filesystemId: string, recipientUsername: string, accessLevel = 'viewer') {
	const res = await fetch('http://localhost:3000/send_share_request', {
		method: 'POST',
		credentials: 'include',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ filesystem_id: filesystemId, recipient_username: recipientUsername, access_level: accessLevel })
	});
	if (!res.ok) throw new Error(await res.text());
}

export async function acceptShareRequest(id: string) {
	const res = await fetch('http://localhost:3000/accept_share_request', {
		method: 'POST',
		credentials: 'include',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ id })
	});
	if (!res.ok) throw new Error(await res.text());
}

export async function declineShareRequest(id: string) {
	const res = await fetch('http://localhost:3000/decline_share_request', {
		method: 'POST',
		credentials: 'include',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ id })
	});
	if (!res.ok) throw new Error(await res.text());
}

export async function revokePermission(id: string) {
	const res = await fetch('http://localhost:3000/revoke_permission', {
		method: 'POST',
		credentials: 'include',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ id })
	});
	if (!res.ok) throw new Error(await res.text());
}

export async function fetchMyGrants(): Promise<PermissionEntry[]> {
	const res = await fetch('http://localhost:3000/my_grants', { credentials: 'include' });
	if (!res.ok) throw new Error(await res.text());
	return res.json();
}

export async function fetchSharedFolder(permissionFilesystemId: string, parentId?: string | null): Promise<Folder[]> {
	let url = `http://localhost:3000/shared_folder?permission_filesystem_id=${permissionFilesystemId}`;
	if (parentId) url += `&parent_id=${parentId}`;
	const res = await fetch(url, { credentials: 'include' });
	if (!res.ok) throw new Error(await res.text());
	return res.json();
}

export async function fetchSharedFiles(permissionFilesystemId: string, parentId?: string | null): Promise<FileEntry[]> {
	let url = `http://localhost:3000/shared_files?permission_filesystem_id=${permissionFilesystemId}`;
	if (parentId) url += `&parent_id=${parentId}`;
	const res = await fetch(url, { credentials: 'include' });
	if (!res.ok) throw new Error(await res.text());
	return res.json();
}

export async function copySharedFile(filesystemId: string, parentId?: string | null) {
	const res = await fetch('http://localhost:3000/copy_shared_file', {
		method: 'POST',
		credentials: 'include',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ filesystem_id: filesystemId, parent_id: parentId ?? null })
	});
	if (!res.ok) throw new Error(await res.text());
}
