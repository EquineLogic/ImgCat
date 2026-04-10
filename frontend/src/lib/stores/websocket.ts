import { pendingRequests, fetchPendingRequests, fetchSharedWithMe, sharedWithMe } from './sharing';
import { WS_BASE } from '$lib/config';

type SharingEvent =
	| { event: 'NewShareRequest'; request_id: string; filesystem_id: string; entry_name: string; sender_username: string; access_level: string }
	| { event: 'ShareRequestAccepted'; request_id: string; filesystem_id: string; recipient_username: string }
	| { event: 'ShareRequestDeclined'; request_id: string; recipient_username: string }
	| { event: 'ShareRequestCancelled'; request_id: string }
	| { event: 'PermissionRevoked'; permission_id: string; filesystem_id: string }
	| { event: 'Lagged'; missed: number };

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
	};

	ws.onmessage = (ev) => {
		try {
			const event: SharingEvent = JSON.parse(ev.data);
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

function handleEvent(event: SharingEvent) {
	switch (event.event) {
		case 'NewShareRequest':
			// Re-fetch to get full data with presigned URLs
			fetchPendingRequests();
			break;

		case 'ShareRequestAccepted':
			window.dispatchEvent(new CustomEvent('sharing:accepted', { detail: event }));
			break;

		case 'ShareRequestDeclined':
			window.dispatchEvent(new CustomEvent('sharing:declined', { detail: event }));
			break;

		case 'ShareRequestCancelled':
			pendingRequests.update((reqs) => reqs.filter((r) => r.id !== event.request_id));
			break;

		case 'PermissionRevoked':
			sharedWithMe.update((items) => items.filter((i) => i.id !== event.permission_id));
			break;

		case 'Lagged':
			// Missed events -- full refresh
			fetchPendingRequests();
			fetchSharedWithMe();
			break;
	}
}
