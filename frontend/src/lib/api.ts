import { API_BASE } from './config';

export async function op<T = unknown>(args: object, anon = false): Promise<T> {
	const res = await fetch(`${API_BASE}/${anon ? 'op_anon' : 'op_auth'}`, {
		method: 'POST',
		credentials: 'include',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(args)
	});
	if (!res.ok) throw new Error(await res.text());
	return res.json() as Promise<T>;
}
