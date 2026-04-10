<script lang="ts">
	import { onMount } from 'svelte';
	import { API_BASE } from '$lib/config';

	const options = [
		{ value: 0, label: 'Never (keep trash forever)' },
		{ value: 7, label: '7 days' },
		{ value: 14, label: '14 days' },
		{ value: 30, label: '30 days' },
		{ value: 60, label: '60 days' },
		{ value: 90, label: '90 days' }
	];

	let selected = $state(30);
	let isLoaded = $state(false);
	let saving = $state(false);
	let error = $state('');
	let message = $state('');

	$inspect(selected);

	async function load() {
		if (isLoaded) return;
		try {
			const res = await fetch(`${API_BASE}/trash_retention`, {
				credentials: 'include'
			});
			if (res.ok) {
				const data = await res.json();
				selected = data.days === 0 ? 0 : Number(data.days) || 30;
			}
		} catch (e) {
			// keep default
		} finally {
			isLoaded = true;
		}
	}

	onMount(load);

	async function save() {
		saving = true;
		error = '';
		message = '';
		try {
			const res = await fetch(`${API_BASE}/trash_retention`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				credentials: 'include',
				body: JSON.stringify({ days: selected })
			});
			if (!res.ok) {
				error = await res.text();
				return;
			}
			message = 'Saved';
		} catch (e) {
			error = 'Request failed';
		} finally {
			saving = false;
		}
	}
</script>

<div class="p-8 max-w-3xl">
	<h1 class="text-xl font-semibold text-white mb-6">Settings</h1>

	<section class="rounded-2xl border border-white/10 bg-tw-darkblue/40 p-6">
		<h2 class="text-sm uppercase tracking-wider text-white/40 mb-2">Trash Cleanup</h2>
		<p class="text-sm text-white/50 mb-5">
			Automatically purge items from the trash after this many days.
		</p>

		<div class="flex flex-col gap-2 mb-5">
			{#each options as opt}
				<button
					type="button"
					onclick={() => (selected = opt.value)}
					class={`w-full text-center px-4 py-3 rounded-xl cursor-pointer border transition-colors duration-150 ${
						isLoaded && selected === opt.value
							? 'border-tw-yellow bg-tw-yellow/20 text-tw-yellow'
							: 'border-white/10 hover:border-white/20 hover:bg-white/5 text-white/90'
					}`}
				>
					<span class="text-sm">{opt.label}</span>
				</button>
			{/each}
		</div>

		{#if error}
			<p class="text-sm text-red-400 mb-3">{error}</p>
		{/if}
		{#if message}
			<p class="text-sm text-green-400 mb-3">{message}</p>
		{/if}

		<button
			onclick={save}
			disabled={saving}
			class="w-full rounded-xl py-3 font-semibold text-white
				       bg-linear-to-br from-tw-purple to-tw-pink
				       transition-all duration-200
				       focus:outline-none focus:ring-2 focus:ring-tw-neon
				       {saving
				? 'opacity-50 cursor-not-allowed'
				: 'hover:shadow-[0_0_20px_rgba(237,67,141,0.4)] cursor-pointer'}"
		>
			{saving ? 'Saving...' : 'Save'}
		</button>
	</section>
</div>
