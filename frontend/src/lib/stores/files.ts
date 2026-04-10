import { writable, get } from 'svelte/store';
import { currentFolderId } from './folders';
import { API_BASE } from '$lib/config';

export type FileEntry = { id: string; name: string; mime_type: string; size_bytes: number; url: string; };

export const files = writable<FileEntry[]>([]);

export async function fetchFiles(parentId?: string | null) {
	const id = parentId !== undefined ? parentId : get(currentFolderId);
	const url = id
		? `${API_BASE}/list_files?parent_id=${id}`
		: `${API_BASE}/list_files`;
	const res = await fetch(url, { credentials: 'include' });
	if (res.ok) {
		files.set(await res.json());
	}
}
