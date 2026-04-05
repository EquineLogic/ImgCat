<script lang="ts">
	import Modal from './Modal.svelte';

	let {
		open = $bindable(false),
		title,
		currentName,
		onSubmit
	} = $props<{
		open: boolean;
		title: string;
		currentName: string;
		onSubmit: (newName: string) => Promise<string | null>;
	}>();

	let newName = $state('');
	let error = $state('');
	let submitting = $state(false);

	$effect(() => {
		if (open) {
			newName = currentName;
			error = '';
		}
	});

	async function handleSubmit() {
		const trimmed = newName.trim();
		if (!trimmed || trimmed === currentName) {
			open = false;
			return;
		}
		submitting = true;
		const err = await onSubmit(trimmed);
		submitting = false;
		if (err) {
			error = err;
			return;
		}
		error = '';
		open = false;
	}
</script>

<Modal bind:open {title}>
	<form
		onsubmit={(e) => {
			e.preventDefault();
			handleSubmit();
		}}
		class="flex flex-col gap-4"
	>
		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">New Name</span>
			<input
				type="text"
				bind:value={newName}
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80
				       border border-tw-purple/40 text-white
				       placeholder:text-white/30
				       focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>
		{#if error}
			<p class="text-sm text-red-400">{error}</p>
		{/if}
		<button
			type="submit"
			disabled={!newName.trim() || newName.trim() === currentName || submitting}
			class="rounded-lg py-2.5 font-semibold text-white
			       transition-colors duration-200
			       focus:outline-none focus:ring-2 focus:ring-tw-neon
			       {newName.trim() && newName.trim() !== currentName && !submitting
				? 'bg-tw-purple hover:bg-tw-pink cursor-pointer'
				: 'bg-white/10 text-white/30 cursor-not-allowed'}"
		>
			{submitting ? 'Saving...' : 'Rename'}
		</button>
	</form>
</Modal>
