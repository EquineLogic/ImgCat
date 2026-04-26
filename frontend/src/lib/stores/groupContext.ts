import { writable, get } from 'svelte/store';

export type GroupContext = { group_id: string; group_username: string };

const STORAGE_KEY = 'group_context';

function loadInitial(): GroupContext | null {
	if (typeof localStorage === 'undefined') return null;
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		return raw ? JSON.parse(raw) : null;
	} catch {
		return null;
	}
}

export const groupContext = writable<GroupContext | null>(loadInitial());

groupContext.subscribe((v) => {
	if (typeof localStorage === 'undefined') return;
	if (v) localStorage.setItem(STORAGE_KEY, JSON.stringify(v));
	else localStorage.removeItem(STORAGE_KEY);
});

export function currentGroupId(): string | null {
	return get(groupContext)?.group_id ?? null;
}
