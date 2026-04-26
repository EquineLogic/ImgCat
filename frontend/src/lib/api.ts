import { API_BASE } from './config';
import { groupContext, currentGroupId } from './stores/groupContext';

export async function op<T = unknown>(args: object, anon = false): Promise<T> {
	const headers: Record<string, string> = { 'Content-Type': 'application/json' };
	if (!anon) {
		const gid = currentGroupId();
		if (gid) headers['X-Group'] = gid;
	}
	const res = await fetch(`${API_BASE}/${anon ? 'op_anon' : 'op_auth'}`, {
		method: 'POST',
		credentials: 'include',
		headers,
		body: JSON.stringify(args)
	});
	if (!res.ok) {
		const text = await res.text();
		// Auto-recover: backend says the X-Group we're sending isn't a valid
		// membership for this user. Drop the context so subsequent calls run
		// in personal mode and the user can re-pick from /home/groups.
		if (res.status === 401 && text.includes('selected group')) {
			groupContext.set(null);
		}
		throw new Error(text);
	}
	return res.json() as Promise<T>;
}
