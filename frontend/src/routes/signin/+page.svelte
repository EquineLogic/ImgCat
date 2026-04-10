<script lang="ts">
	import { goto } from '$app/navigation';
	import { user } from '$lib/stores/auth';
	import { API_BASE } from '$lib/config';

	let message = $state('');
	let isError = $state(false);

	async function handleSignIn(e: SubmitEvent) {
		e.preventDefault();
		const form = e.target as HTMLFormElement;
		const formData = new FormData(form);

		const res = await fetch(`${API_BASE}/signin`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({
				username: formData.get('username'),
				password: formData.get('password')
			})
		});

		if (res.ok) {
			const data = await res.json();
			user.set(data);
			goto('/home');
			return;
		}

		const text = await res.text();
		message = text;
		isError = true;
	}
</script>

<div class="min-h-screen bg-tw-darkblue flex flex-col items-center justify-center px-4 pb-20">
	<a
		href="/"
		class="text-6xl sm:text-7xl font-extrabold mb-14 pb-2
               leading-normal
               bg-linear-to-r from-tw-purple to-tw-pink
               bg-clip-text text-transparent
               drop-shadow-[0_0_12px_rgba(0,245,255,0.5)]
               no-underline hover:drop-shadow-[0_0_20px_rgba(0,245,255,0.7)]
               transition-all duration-300"
	>
		ImgCat
	</a>

	<form
		onsubmit={handleSignIn}
		class="w-full max-w-sm bg-white/5 backdrop-blur
             border border-tw-purple/30 rounded-2xl p-8
             flex flex-col gap-5"
	>
		<h2 class="text-tw-neon text-xl font-semibold text-center">Sign In</h2>

		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Username</span>
			<input
				type="text"
				name="username"
				required
				placeholder="twilight_sparkle"
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
                 border border-tw-purple/40 text-white
                 placeholder:text-white/30
                 focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>

		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Password</span>
			<input
				type="password"
				name="password"
				required
				placeholder="********"
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
                 border border-tw-purple/40 text-white
                 placeholder:text-white/30
                 focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>

		<button
			type="submit"
			class="mt-2 rounded-lg py-2.5 font-semibold text-white
               bg-tw-purple hover:bg-tw-pink cursor-pointer
               transition-colors duration-200
               focus:outline-none focus:ring-2 focus:ring-tw-neon"
		>
			Sign In
		</button>

		{#if message}
			<p class="text-sm text-center {isError ? 'text-red-400' : 'text-green-400'}">{message}</p>
		{/if}

		<p class="text-tw-yellow text-lg text-center">
			Don't have an account?
			<a
				href="/register"
				class="text-tw-neon font-semibold hover:text-tw-pink transition-colors duration-200"
			>
				Register
			</a>
		</p>
	</form>
</div>
