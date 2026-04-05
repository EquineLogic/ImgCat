<script lang="ts">
	import type { Snippet } from 'svelte';
	import Modal from './Modal.svelte';

	let {
		open = $bindable(false),
		title,
		confirmLabel = 'Confirm',
		danger = false,
		onConfirm,
		children
	} = $props<{
		open: boolean;
		title: string;
		confirmLabel?: string;
		danger?: boolean;
		onConfirm: () => void | Promise<void>;
		children?: Snippet;
	}>();

	let submitting = $state(false);

	async function handleConfirm() {
		submitting = true;
		await onConfirm();
		submitting = false;
		open = false;
	}
</script>

<Modal bind:open {title}>
	<div class="flex flex-col gap-4">
		{#if children}
			{@render children()}
		{/if}
		<div class="flex gap-2 justify-end">
			<button
				onclick={() => (open = false)}
				class="px-4 py-2 rounded-lg text-sm text-white/60 hover:text-white hover:bg-white/10
				       cursor-pointer transition-colors duration-150"
			>
				Cancel
			</button>
			<button
				onclick={handleConfirm}
				disabled={submitting}
				class="px-4 py-2 rounded-lg text-sm font-semibold text-white
				       cursor-pointer transition-colors duration-150
				       {danger
					? 'bg-red-500/80 hover:bg-red-500'
					: 'bg-tw-purple hover:bg-tw-pink'}
				       {submitting ? 'opacity-50 cursor-not-allowed' : ''}"
			>
				{submitting ? '...' : confirmLabel}
			</button>
		</div>
	</div>
</Modal>
