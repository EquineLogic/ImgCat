<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { user } from '$lib/stores/auth';
	import { connectWebSocket, disconnectWebSocket } from '$lib/stores/websocket';
	import favicon from '$lib/assets/favicon.svg';
	import "../app.css";

	let { children } = $props();

	const publicRoutes = ['/', '/signin', '/register'];

	onMount(async () => {
		try {
			const res = await fetch('http://localhost:3000/check_auth', {
				credentials: 'include'
			});

			if (res.ok) {
				const data = await res.json();
				user.set(data);
				connectWebSocket();
				if (publicRoutes.includes(page.url.pathname)) {
					goto('/home');
				}
			} else {
				user.set(null);
				disconnectWebSocket();
				if (!publicRoutes.includes(page.url.pathname)) {
					goto('/signin');
				}
			}
		} catch {
			user.set(null);
			disconnectWebSocket();
			if (!publicRoutes.includes(page.url.pathname)) {
				goto('/signin');
			}
		}
	});

	onDestroy(() => {
		disconnectWebSocket();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
