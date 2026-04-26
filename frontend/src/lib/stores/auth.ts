import { writable } from 'svelte/store';

export type AuthUser = { user_id: string; username: string; session_id: string };

export const user = writable<AuthUser | null>(null);