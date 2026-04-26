import { writable } from 'svelte/store';

export type AuthUser = { username: string; session_id: string };

export const user = writable<AuthUser | null>(null);
