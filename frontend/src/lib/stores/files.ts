import { writable, get } from 'svelte/store';
import { currentFolderId } from './folders';

export type FileEntry = { id: string; name: string; mime_type: string; size_bytes: number; url: string; };

export const files = writable<FileEntry[]>([]);

export async function fetchFiles(parentId?: string | null) {
	const id = parentId !== undefined ? parentId : get(currentFolderId);
	const url = id 
		? `http://localhost:3000/list_files?parent_id=${id}`
		: 'http://localhost:3000/list_files';
	const res = await fetch(url, { credentials: 'include' });
	if (res.ok) {
		files.set(await res.json());
	}
}
