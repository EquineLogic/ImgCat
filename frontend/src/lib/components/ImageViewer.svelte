<script lang="ts">
	import Modal from './Modal.svelte';
	import { fetchFiles } from '$lib/stores/files';

	let { open = $bindable(false), id, name } = $props<{
		open: boolean;
		id: string;
		name: string;
	}>();

	// Zoom / pan state
	let scale = $state(1);
	let tx = $state(0);
	let ty = $state(0);

	// Pan gesture
	let panning = $state(false);
	let panStartX = 0;
	let panStartY = 0;
	let txStart = 0;
	let tyStart = 0;

	// UI state
	let menuOpen = $state(false);
	let showRename = $state(false);
	let newName = $state('');
	let renameError = $state('');
	let deleting = $state(false);

	function close() {
		open = false;
		// Reset viewer state for next open
		scale = 1;
		tx = 0;
		ty = 0;
		menuOpen = false;
	}

	function onKeydown(e: KeyboardEvent) {
		if (!open) return;
		if (e.key === 'Escape') {
			if (menuOpen) menuOpen = false;
			else if (showRename || deleting) {
				// let modals handle their own esc
			} else close();
		} else if (e.key === '0') {
			scale = 1;
			tx = 0;
			ty = 0;
		}
	}

	function onWheel(e: WheelEvent) {
		e.preventDefault();
		// Scroll up (negative deltaY) -> zoom in
		const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
		const next = Math.max(1, Math.min(8, scale * factor));
		scale = next;
		if (scale === 1) {
			tx = 0;
			ty = 0;
		}
	}

	function onPanStart(e: PointerEvent) {
		if (scale === 1) return;
		if (e.button !== 0) return;
		panning = true;
		panStartX = e.clientX;
		panStartY = e.clientY;
		txStart = tx;
		tyStart = ty;
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}

	function onPanMove(e: PointerEvent) {
		if (!panning) return;
		tx = txStart + (e.clientX - panStartX);
		ty = tyStart + (e.clientY - panStartY);
	}

	function onPanEnd(e: PointerEvent) {
		panning = false;
		try {
			(e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
		} catch {}
	}

	function onBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) close();
	}

	async function download() {
		menuOpen = false;
		const res = await fetch(`http://localhost:3000/files/${id}`, { credentials: 'include' });
		if (!res.ok) return;
		const blob = await res.blob();
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = name;
		document.body.appendChild(a);
		a.click();
		a.remove();
		URL.revokeObjectURL(url);
	}

	function openRename() {
		menuOpen = false;
		newName = name;
		renameError = '';
		showRename = true;
	}

	async function submitRename() {
		const trimmed = newName.trim();
		if (!trimmed || trimmed === name) {
			showRename = false;
			return;
		}
		const res = await fetch('http://localhost:3000/rename_file', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({ id, name: trimmed })
		});
		if (!res.ok) {
			renameError = await res.text();
			return;
		}
		renameError = '';
		showRename = false;
		await fetchFiles();
	}

	async function confirmDelete() {
		deleting = true;
		menuOpen = false;
	}

	async function submitDelete() {
		const res = await fetch('http://localhost:3000/delete_file', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			credentials: 'include',
			body: JSON.stringify({ id })
		});
		deleting = false;
		if (res.ok) {
			await fetchFiles();
			close();
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="fixed inset-0 z-40 bg-black/90 backdrop-blur-sm flex items-center justify-center overflow-hidden"
		onclick={onBackdropClick}
		onwheel={onWheel}
	>
		<!-- Top bar -->
		<div class="absolute top-0 left-0 right-0 z-10 flex items-center justify-between px-6 py-4
		            bg-gradient-to-b from-black/70 to-transparent pointer-events-none">
			<span class="text-white/90 text-sm font-medium truncate max-w-[50%]">{name}</span>
			<div class="flex items-center gap-2 pointer-events-auto">
				<!-- Zoom hint -->
				{#if scale !== 1}
					<span class="text-white/40 text-xs mr-2">{Math.round(scale * 100)}%</span>
				{/if}

				<!-- Options menu -->
				<div class="relative">
					<button
						onclick={(e) => { e.stopPropagation(); menuOpen = !menuOpen; }}
						aria-label="Options"
						class="w-9 h-9 flex items-center justify-center rounded-lg
						       text-white/60 hover:text-white hover:bg-white/10
						       cursor-pointer transition-colors duration-150"
					>
						<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5">
							<circle cx="5" cy="12" r="2" />
							<circle cx="12" cy="12" r="2" />
							<circle cx="19" cy="12" r="2" />
						</svg>
					</button>
					{#if menuOpen}
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="absolute right-0 top-11 flex flex-col py-1.5 min-w-[160px]
							       bg-tw-darkblue/95 backdrop-blur-xl border border-white/10
							       rounded-xl shadow-[0_8px_30px_rgba(0,0,0,0.5)] overflow-hidden"
							onclick={(e) => e.stopPropagation()}
						>
							<button
								onclick={download}
								class="flex items-center gap-2.5 px-3.5 py-2 text-sm text-white/70 hover:text-white hover:bg-white/10 cursor-pointer transition-colors duration-150"
							>
								<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" class="w-4 h-4">
									<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
									<polyline points="7 10 12 15 17 10" />
									<line x1="12" y1="15" x2="12" y2="3" />
								</svg>
								Download
							</button>
							<button
								onclick={openRename}
								class="flex items-center gap-2.5 px-3.5 py-2 text-sm text-white/70 hover:text-white hover:bg-white/10 cursor-pointer transition-colors duration-150"
							>
								<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" class="w-4 h-4">
									<path d="M17 3a2.83 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
									<path d="m15 5 4 4" />
								</svg>
								Rename
							</button>
							<button
								onclick={confirmDelete}
								class="flex items-center gap-2.5 px-3.5 py-2 text-sm text-red-400 hover:bg-red-400/10 cursor-pointer transition-colors duration-150"
							>
								<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" class="w-4 h-4">
									<path d="M3 6h18" />
									<path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
									<path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
								</svg>
								Delete
							</button>
						</div>
					{/if}
				</div>

				<!-- Close button -->
				<button
					onclick={close}
					aria-label="Close"
					class="w-9 h-9 flex items-center justify-center rounded-lg
					       text-white/60 hover:text-white hover:bg-white/10
					       cursor-pointer transition-colors duration-150"
				>
					<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-5 h-5">
						<path d="M18 6 6 18M6 6l12 12" />
					</svg>
				</button>
			</div>
		</div>

		<!-- Image wrapper (pan gestures) -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="w-full h-full flex items-center justify-center select-none"
			style="cursor: {scale > 1 ? (panning ? 'grabbing' : 'grab') : 'default'}"
			onpointerdown={onPanStart}
			onpointermove={onPanMove}
			onpointerup={onPanEnd}
			onpointercancel={onPanEnd}
			onclick={(e) => e.stopPropagation()}
		>
			<img
				src="http://localhost:3000/files/{id}"
				alt={name}
				draggable="false"
				class="max-w-[90vw] max-h-[90vh] object-contain {panning ? '' : 'transition-transform duration-75'}"
				style="transform: translate({tx}px, {ty}px) scale({scale})"
			/>
		</div>
	</div>
{/if}

<!-- Rename modal -->
<Modal bind:open={showRename} title="Rename File">
	<form onsubmit={(e) => { e.preventDefault(); submitRename(); }} class="flex flex-col gap-4">
		<label class="flex flex-col gap-1">
			<span class="text-tw-yellow text-sm">New Name</span>
			<input
				type="text"
				bind:value={newName}
				class="rounded-lg px-4 py-2.5 bg-tw-darkblue/80 border border-tw-purple/40 text-white
				       placeholder:text-white/30 focus:outline-none focus:ring-2 focus:ring-tw-neon"
			/>
		</label>
		{#if renameError}
			<p class="text-sm text-red-400">{renameError}</p>
		{/if}
		<button
			type="submit"
			disabled={!newName.trim() || newName.trim() === name}
			class="rounded-lg py-2.5 font-semibold text-white transition-colors duration-200
			       focus:outline-none focus:ring-2 focus:ring-tw-neon
			       {newName.trim() && newName.trim() !== name ? 'bg-tw-purple hover:bg-tw-pink cursor-pointer' : 'bg-white/10 text-white/30 cursor-not-allowed'}"
		>
			Rename
		</button>
	</form>
</Modal>

<!-- Delete confirmation modal -->
<Modal bind:open={deleting} title="Delete File">
	<div class="flex flex-col gap-4">
		<p class="text-sm text-white/70">
			Move <span class="text-white font-medium">{name}</span> to trash? You can restore it later.
		</p>
		<div class="flex gap-2 justify-end">
			<button
				onclick={() => (deleting = false)}
				class="px-4 py-2 rounded-lg text-sm text-white/60 hover:text-white hover:bg-white/10 cursor-pointer transition-colors duration-150"
			>
				Cancel
			</button>
			<button
				onclick={submitDelete}
				class="px-4 py-2 rounded-lg text-sm font-semibold text-white bg-red-500/80 hover:bg-red-500 cursor-pointer transition-colors duration-150"
			>
				Delete
			</button>
		</div>
	</div>
</Modal>
