import { getToken } from '$lib/api';
import { WS_BASE } from '$lib/config';
import { fetchGroups } from './groups';

type GroupEvent =
	| { event: 'NewGroupInvite'; group_id: string }
	| { event: 'AcceptedGroupInvite'; group_id: string; user_id: string }
	| { event: 'DeniedGroupInvite'; group_id: string; user_id: string };

let ws: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectDelay = 1000;
const MAX_RECONNECT_DELAY = 30000;

export function connectWebSocket() {
	if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
		return;
	}

	ws = new WebSocket(`${WS_BASE}/ws`);

	ws.onopen = () => {
		reconnectDelay = 1000;
		
		// identify to server
		if(ws) {
			let token = getToken()
			if (!token) {
				ws.close(4001, "Client has no token")
				return
			}
			ws.send(token)
		}
	};

	ws.onmessage = (ev) => {
		try {
			const event = JSON.parse(ev.data) as GroupEvent;
			handleEvent(event);
		} catch {
			// ignore malformed messages
		}
	};

	ws.onclose = (ev) => {
		ws = null;
		if (ev.code === 4001) {
			// Unauthorized -- don't reconnect
			return;
		}
		scheduleReconnect();
	};

	ws.onerror = () => {
		// onerror is always followed by onclose
	};
}

function scheduleReconnect() {
	if (reconnectTimer) return;
	reconnectTimer = setTimeout(() => {
		reconnectTimer = null;
		reconnectDelay = Math.min(reconnectDelay * 2, MAX_RECONNECT_DELAY);
		connectWebSocket();
	}, reconnectDelay);
}

export function disconnectWebSocket() {
	if (reconnectTimer) {
		clearTimeout(reconnectTimer);
		reconnectTimer = null;
	}
	if (ws) {
		ws.close();
		ws = null;
	}
}

function handleEvent(event: GroupEvent) {
	switch (event.event) {
		case 'NewGroupInvite':
		case 'AcceptedGroupInvite':
		case 'DeniedGroupInvite':
			// All three change membership state visible via ListGroups.
			// fetchGroups() is a no-op when in group context, which is fine —
			// the invite badge / list only matter in personal context anyway.
			fetchGroups();
			break;
	}
}
