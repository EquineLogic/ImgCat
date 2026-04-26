import { WS_BASE } from '$lib/config';

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

	ws.onmessage = () => {
		// Group/sharing events are not yet wired up on the frontend.
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
