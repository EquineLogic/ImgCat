import { writable, get } from 'svelte/store';

export type Folder = { id: string; name: string };
export type BreadcrumbItem = { id: string | null; name: string };

export const folders = writable<Folder[]>([]);
export const currentFolderId = writable<string | null>(null);
export const breadcrumbs = writable<BreadcrumbItem[]>([{ id: null, name: 'My Library' }]);

export async function fetchFolders(parentId?: string | null) {
	const id = parentId !== undefined ? parentId : get(currentFolderId);
	const url = id
		? `http://localhost:3000/list_folders?parent_id=${id}`
		: 'http://localhost:3000/list_folders';
	const res = await fetch(url, { credentials: 'include' });
	if (res.ok) {
		folders.set(await res.json());
	}
}

export function openFolder(id: string, name: string) {
	currentFolderId.set(id);
	breadcrumbs.update((b) => [...b, { id, name }]);
	fetchFolders(id);
}

export function navigateToBreadcrumb(index: number) {
	breadcrumbs.update((b) => {
		const sliced = b.slice(0, index + 1);
		const target = sliced[sliced.length - 1];
		currentFolderId.set(target.id);
		fetchFolders(target.id);
		return sliced;
	});
}
