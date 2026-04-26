import { writable, get } from 'svelte/store';
import { currentFolderId } from './folders';
import { op } from '$lib/api';

export type FileEntry = { id: string; name: string; mime_type: string; size_bytes: number; url: string; };

export const files = writable<FileEntry[]>([]);

export async function fetchFiles(parentId?: string | null) {
	const id = parentId !== undefined ? parentId : get(currentFolderId);
	try {
		const r = await op<{ op: 'Files'; files: FileEntry[] }>({ op: 'ListFiles', parent_id: id ?? null });
		files.set(r.files);
	} catch {
		// leave existing list as-is on failure
	}
}
