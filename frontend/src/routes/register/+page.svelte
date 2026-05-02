<script lang="ts">
	import { goto } from '$app/navigation';
	import { user } from '$lib/stores/auth';
	import { fetchClient, op, setToken } from '$lib/api';
	import { API_BASE } from '$lib/config';

	let message = $state('');
	let isError = $state(false);
	let password = $state('');
	let username = $state('');
	let displayName = $state('');

	const usernameValid = $derived(username.length >= 5);
	const displayNameValid = $derived(displayName.length >= 5);

	const rules = $derived([
		{ label: 'At least 12 characters', pass: password.length >= 12 },
		{ label: 'At least 1 uppercase letter', pass: /[A-Z]/.test(password) },
		{ label: 'At least 1 lowercase letter', pass: /[a-z]/.test(password) },
		{ label: 'At least 1 number', pass: /[0-9]/.test(password) },
		{ label: 'At least 1 special character', pass: /[^A-Za-z0-9]/.test(password) }
	]);

	const allValid = $derived(rules.every((r) => r.pass));

	async function handleRegister(e: SubmitEvent) {
		e.preventDefault();

		if (!displayNameValid) {
			message = 'Display name must be at least 5 characters';
			isError = true;
			return;
		}

		if (!usernameValid) {
			message = 'Username must be at least 5 characters';
			isError = true;
			return;
		}

		if (!allValid) {
			message = 'Please meet all password requirements';
			isError = true;
			return;
		}

		const form = e.target as HTMLFormElement;
		const formData = new FormData(form);

		try {
			let resp: {token: string} = await op(
				{
					op: 'CreateUser',
					username: formData.get('username'),
					name: formData.get('name'),
					password: formData.get('password')
				},
				true
			);
			if (!resp.token) throw new Error("No token returned by OP")
			setToken(resp.token)

			const meRes = await fetchClient(`${API_BASE}/check_auth`);
			if (!meRes.ok) throw new Error(await meRes.text());
			const me = await meRes.json();
			user.set({ 
				user_id: me.user_id, 
				username: me.username, 
				session_id: me.session_id,
				preferences: me.preferences
			});
			goto('/home');
		} catch (e: any) {
			message = e?.message || 'Registration failed';
			isError = true;
		}
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
		onsubmit={handleRegister}
		class="w-full max-w-sm bg-white/5 backdrop-blur
             border border-tw-purple/30 rounded-2xl p-8
             flex flex-col gap-5"
	>
		<h2 class="text-tw-neon text-xl font-semibold text-center">Register</h2>

		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Username</span>
			<input
				type="text"
				name="username"
				required
				bind:value={username}
				placeholder="twilight_sparkle"
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
                 border border-tw-purple/40 text-white
                 placeholder:text-white/30
                 focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
			{#if username.length > 0 && !usernameValid}
				<span class="text-xs text-white/40">{'\u2717'} At least 5 characters</span>
			{/if}
		</label>

		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Display Name</span>
			<input
				type="text"
				name="name"
				required
				bind:value={displayName}
				placeholder="Twilight Sparkle"
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
                 border border-tw-purple/40 text-white
                 placeholder:text-white/30
                 focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
			{#if displayName.length > 0 && !displayNameValid}
				<span class="text-xs text-white/40">{'\u2717'} At least 5 characters</span>
			{/if}
		</label>

		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Password</span>
			<input
				type="password"
				name="password"
				required
				bind:value={password}
				placeholder="********"
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
                 border border-tw-purple/40 text-white
                 placeholder:text-white/30
                 focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>

		{#if password.length > 0}
			<ul class="flex flex-col gap-1 text-xs">
				{#each rules as rule}
					<li class={rule.pass ? 'text-green-400' : 'text-white/40'}>
						{rule.pass ? '\u2713' : '\u2717'}
						{rule.label}
					</li>
				{/each}
			</ul>
		{/if}

		<button
			type="submit"
			class="mt-2 rounded-lg py-2.5 font-semibold text-white
               bg-tw-purple hover:bg-tw-pink cursor-pointer
               transition-colors duration-200
               focus:outline-none focus:ring-2 focus:ring-tw-neon"
		>
			Register
		</button>

		{#if message}
			<p class="text-sm text-center {isError ? 'text-red-400' : 'text-green-400'}">{message}</p>
		{/if}

		<p class="text-tw-yellow text-lg text-center">
			Already have an account?
			<a
				href="/signin"
				class="text-tw-neon font-semibold hover:text-tw-pink transition-colors duration-200"
			>
				Sign In
			</a>
		</p>
	</form>
</div>
