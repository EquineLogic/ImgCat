import { API_BASE } from './config';
import { groupContext, currentGroupId } from './stores/groupContext';

export const getToken = (): string | null => {
	let tok = localStorage.getItem("usertoken")
	if (!tok) return null
	return tok
}

export const setToken = (token: string) => {
	if (!token) throw new Error("token is null to setToken")
	console.log("setToken()")
	localStorage.setItem("usertoken", token)
}

export async function op<T = unknown>(args: object, anon = false): Promise<T> {
	const headers: Record<string, string> = { 'Content-Type': 'application/json' };
	if (!anon) {
		const gid = currentGroupId();
		if (gid) {
			headers['X-Group'] = gid;
		}

		let token = getToken()
		if (!token) {
			throw new Error("Cannot execute an authenticated action without a valid session")
		}
		headers["Authorization"] = token
	}
	const res = await fetch(`${API_BASE}/${anon ? 'op_anon' : 'op_auth'}`, {
		method: 'POST',
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

export async function fetchClient(url: string, init?: RequestInit) {
	let patchedInit = init || {}
	let newHeaders = patchedInit.headers || {}
	// @ts-ignore
	newHeaders["Authorization"] = getToken()
	patchedInit.headers = newHeaders
	return fetch(url, init)
}