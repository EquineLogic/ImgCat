import { writable, get } from 'svelte/store';
import { fetchFiles } from './files';
import { op } from '$lib/api';

export type Folder = { id: string; name: string };
export type BreadcrumbItem = { id: string | null; name: string };

function loadSaved(): { folderId: string | null; breadcrumbs: BreadcrumbItem[] } {
	try {
		const saved = sessionStorage.getItem('nav_state');
		if (saved) return JSON.parse(saved);
	} catch {}
	return { folderId: null, breadcrumbs: [{ id: null, name: 'My Library' }] };
}

function saveState() {
	sessionStorage.setItem(
		'nav_state',
		JSON.stringify({ folderId: get(currentFolderId), breadcrumbs: get(breadcrumbs) })
	);
}

const saved = loadSaved();
export const folders = writable<Folder[]>([]);
export const currentFolderId = writable<string | null>(saved.folderId);
export const breadcrumbs = writable<BreadcrumbItem[]>(saved.breadcrumbs);

export async function fetchFolders(parentId?: string | null) {
	const id = parentId !== undefined ? parentId : get(currentFolderId);
	try {
		const r = await op<{ op: 'Folders'; folders: Folder[] }>({ op: 'ListFolder', parent_id: id ?? null });
		folders.set(r.folders);
	} catch {
		// leave existing list as-is on failure
	}
}

export function resetToRoot() {
	currentFolderId.set(null);
	breadcrumbs.set([{ id: null, name: 'My Library' }]);
	saveState();
}

export function openFolder(id: string, name: string) {
	currentFolderId.set(id);
	breadcrumbs.update((b) => [...b, { id, name }]);
	saveState();
	fetchFolders(id);
	fetchFiles(id);
}

export function navigateToBreadcrumb(index: number) {
	breadcrumbs.update((b) => {
		const sliced = b.slice(0, index + 1);
		const target = sliced[sliced.length - 1];
		currentFolderId.set(target.id);
		fetchFolders(target.id);
		fetchFiles(target.id);
		return sliced;
	});
	saveState();
}
