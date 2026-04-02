<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { user } from '$lib/stores/auth';
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
				if (publicRoutes.includes(page.url.pathname)) {
					goto('/home');
				}
			} else {
				user.set(null);
				if (!publicRoutes.includes(page.url.pathname)) {
					goto('/signin');
				}
			}
		} catch {
			user.set(null);
			if (!publicRoutes.includes(page.url.pathname)) {
				goto('/signin');
			}
		}
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
