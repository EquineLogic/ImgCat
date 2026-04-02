import { writable } from 'svelte/store';

export type Folder = { id: string; name: string };

export const folders = writable<Folder[]>([]);

export async function fetchFolders() {
	const res = await fetch('http://localhost:3000/list_folders', {
		credentials: 'include'
	});
	if (res.ok) {
		folders.set(await res.json());
	}
}
