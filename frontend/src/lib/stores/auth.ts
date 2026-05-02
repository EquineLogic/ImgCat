import { writable } from 'svelte/store';
import { op } from '$lib/api';

export type UserPreferences = {
	breadcrumb_size: number;
};

export type AuthUser = { 
	user_id: string; 
	username: string; 
	session_id: string;
	preferences: UserPreferences;
};

export const user = writable<AuthUser | null>(null);

export async function updatePreference<K extends keyof UserPreferences>(key: K, value: UserPreferences[K]) {
	user.update((u) => {
		if (!u) return u;
		const newPrefs = { ...u.preferences, [key]: value };
		
		// Persist to backend
		op({ op: 'SetPreferences', preferences: newPrefs }).catch(() => {
			// Handle error if needed, maybe revert? 
			// For now we assume success for optimistic UI
		});

		return { ...u, preferences: newPrefs };
	});
}
