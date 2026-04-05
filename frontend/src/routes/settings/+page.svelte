<script lang="ts">
	import { user } from '$lib/stores/auth';
	import Modal from '$lib/components/Modal.svelte';

	let showUsername = $state(false);
	let newUsername = $state('');
	let usernameError = $state('');
	let usernameSaving = $state(false);

	let showPassword = $state(false);
	let currentPassword = $state('');
	let newPassword = $state('');
	let confirmPassword = $state('');
	let passwordError = $state('');
	let passwordSaving = $state(false);

	const passwordRules = $derived([
		{ label: 'At least 12 characters', pass: newPassword.length >= 12 },
		{ label: 'At least 1 uppercase letter', pass: /[A-Z]/.test(newPassword) },
		{ label: 'At least 1 lowercase letter', pass: /[a-z]/.test(newPassword) },
		{ label: 'At least 1 number', pass: /[0-9]/.test(newPassword) },
		{ label: 'At least 1 special character', pass: /[^A-Za-z0-9]/.test(newPassword) }
	]);
	const passwordAllValid = $derived(passwordRules.every((r) => r.pass));
	const passwordsMatch = $derived(
		confirmPassword.length > 0 && newPassword === confirmPassword
	);

	async function submitUsername() {
		if (!newUsername.trim()) return;
		usernameSaving = true;
		usernameError = '';
		try {
			const res = await fetch('http://localhost:3000/change_username', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				credentials: 'include',
				body: JSON.stringify({ username: newUsername.trim() })
			});
			if (!res.ok) {
				usernameError = await res.text();
				return;
			}
			user.update((u) => (u ? { ...u, username: newUsername.trim() } : u));
			newUsername = '';
			showUsername = false;
		} catch (e) {
			usernameError = 'Request failed';
		} finally {
			usernameSaving = false;
		}
	}

	async function submitPassword() {
		if (!currentPassword || !newPassword) return;
		if (newPassword !== confirmPassword) {
			passwordError = 'Passwords do not match';
			return;
		}
		passwordSaving = true;
		passwordError = '';
		try {
			const res = await fetch('http://localhost:3000/change_password', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				credentials: 'include',
				body: JSON.stringify({
					curr_password: currentPassword,
					new_password: newPassword
				})
			});
			if (!res.ok) {
				passwordError = await res.text();
				return;
			}
			currentPassword = '';
			newPassword = '';
			confirmPassword = '';
			showPassword = false;
		} catch (e) {
			passwordError = 'Request failed';
		} finally {
			passwordSaving = false;
		}
	}
</script>

<div class="p-8 max-w-3xl">
	<h1 class="text-xl font-semibold text-white mb-6">Settings</h1>

	<section class="rounded-2xl border border-white/10 bg-tw-darkblue/40 p-6">
		<h2 class="text-sm uppercase tracking-wider text-white/40 mb-4">Profile</h2>

		<div class="flex items-center gap-4 mb-6">
			<div
				class="w-16 h-16 rounded-full bg-linear-to-br from-tw-purple to-tw-pink
				       flex items-center justify-center text-white text-xl font-bold shrink-0"
			>
				{($user?.username ?? '?')[0].toUpperCase()}
			</div>
			<div class="flex flex-col">
				<span class="text-lg font-medium text-white">{$user?.username ?? '—'}</span>
				<span class="text-sm text-white/40">Signed in</span>
			</div>
		</div>

		<dl class="flex flex-col gap-3">
			<div class="flex items-center justify-between py-2 border-t border-white/5">
				<dt class="text-sm text-white/50">Username</dt>
				<div class="flex items-center gap-3">
					<dd class="text-sm text-white/90">{$user?.username ?? '—'}</dd>
					<button
						onclick={() => {
							newUsername = $user?.username ?? '';
							usernameError = '';
							showUsername = true;
						}}
						class="text-xs px-2.5 py-1 rounded-md text-tw-neon/80 hover:text-tw-neon
						       border border-tw-neon/30 hover:border-tw-neon/60 hover:bg-tw-neon/10
						       cursor-pointer transition-colors duration-150"
					>
						Change
					</button>
				</div>
			</div>
			<div class="flex items-center justify-between py-2 border-t border-white/5">
				<dt class="text-sm text-white/50">Password</dt>
				<div class="flex items-center gap-3">
					<dd class="text-sm text-white/90 tracking-widest">******</dd>
					<button
						onclick={() => {
							currentPassword = '';
							newPassword = '';
							confirmPassword = '';
							passwordError = '';
							showPassword = true;
						}}
						class="text-xs px-2.5 py-1 rounded-md text-tw-neon/80 hover:text-tw-neon
						       border border-tw-neon/30 hover:border-tw-neon/60 hover:bg-tw-neon/10
						       cursor-pointer transition-colors duration-150"
					>
						Change
					</button>
				</div>
			</div>
		</dl>
	</section>
</div>

<Modal bind:open={showUsername} title="Change Username">
	<form
		onsubmit={(e) => {
			e.preventDefault();
			submitUsername();
		}}
		class="flex flex-col gap-4"
	>
		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">New Username</span>
			<input
				type="text"
				bind:value={newUsername}
				placeholder="New username"
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
				       border border-tw-purple/40 text-white
				       placeholder:text-white/30
				       focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>
		{#if usernameError}
			<p class="text-sm text-red-400">{usernameError}</p>
		{/if}
		<button
			type="submit"
			disabled={!newUsername.trim() || usernameSaving}
			class="rounded-lg py-2.5 font-semibold text-white
			       transition-colors duration-200
			       focus:outline-none focus:ring-2 focus:ring-tw-neon
			       {newUsername.trim() && !usernameSaving
				? 'bg-tw-purple hover:bg-tw-pink cursor-pointer'
				: 'bg-white/10 text-white/30 cursor-not-allowed'}"
		>
			{usernameSaving ? 'Saving...' : 'Save'}
		</button>
	</form>
</Modal>

<Modal bind:open={showPassword} title="Change Password">
	<form
		onsubmit={(e) => {
			e.preventDefault();
			submitPassword();
		}}
		class="flex flex-col gap-4"
	>
		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Current Password</span>
			<input
				type="password"
				bind:value={currentPassword}
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
				       border border-tw-purple/40 text-white
				       placeholder:text-white/30
				       focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>
		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">New Password</span>
			<input
				type="password"
				bind:value={newPassword}
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
				       border border-tw-purple/40 text-white
				       placeholder:text-white/30
				       focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>

		{#if newPassword.length > 0}
			<ul class="flex flex-col gap-1 text-xs -mt-2">
				{#each passwordRules as rule}
					<li class={rule.pass ? 'text-green-400' : 'text-white/40'}>
						{rule.pass ? '\u2713' : '\u2717'}
						{rule.label}
					</li>
				{/each}
			</ul>
		{/if}

		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">Confirm New Password</span>
			<input
				type="password"
				bind:value={confirmPassword}
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
				       border border-tw-purple/40 text-white
				       placeholder:text-white/30
				       focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>
		{#if confirmPassword.length > 0 && !passwordsMatch}
			<span class="text-xs text-white/40 -mt-2">{'\u2717'} Passwords do not match</span>
		{/if}
		{#if passwordError}
			<p class="text-sm text-red-400">{passwordError}</p>
		{/if}
		<button
			type="submit"
			disabled={!currentPassword || !passwordAllValid || !passwordsMatch || passwordSaving}
			class="rounded-lg py-2.5 font-semibold text-white
			       transition-colors duration-200
			       focus:outline-none focus:ring-2 focus:ring-tw-neon
			       {currentPassword && passwordAllValid && passwordsMatch && !passwordSaving
				? 'bg-tw-purple hover:bg-tw-pink cursor-pointer'
				: 'bg-white/10 text-white/30 cursor-not-allowed'}"
		>
			{passwordSaving ? 'Saving...' : 'Save'}
		</button>
	</form>
</Modal>
