import { writable } from 'svelte/store';

export type ContextMenuItem = {
	label: string;
	icon: string;
	action: () => void;
	danger?: boolean;
};

export const contextMenu = writable<{
	open: boolean;
	x: number;
	y: number;
	items: ContextMenuItem[];
}>({
	open: false,
	x: 0,
	y: 0,
	items: []
});

export const renameModal = writable<{
	open: boolean;
	title: string;
	currentName: string;
	onSubmit: (newName: string) => Promise<string | null>;
}>({
	open: false,
	title: '',
	currentName: '',
	onSubmit: async () => null
});

export const imageViewer = writable<{
	open: boolean;
	id: string;
	name: string;
	url: string;
	readonly: boolean;
}>({
	open: false,
	id: '',
	name: '',
	url: '',
	readonly: false
});

export const confirmModal = writable<{
	open: boolean;
	title: string;
	message: string;
	confirmLabel: string;
	danger: boolean;
	onConfirm: () => Promise<void>;
}>({
	open: false,
	title: '',
	message: '',
	confirmLabel: '',
	danger: false,
	onConfirm: async () => {}
});

export const uploadModal = writable<{
	open: boolean;
	files: FileList | null;
}>({
	open: false,
	files: null
});

export function openContextMenu(x: number, y: number, items: ContextMenuItem[]) {
	contextMenu.set({ open: true, x, y, items });
}

export function openRenameModal(title: string, currentName: string, onSubmit: (newName: string) => Promise<string | null>) {
	renameModal.set({ open: true, title, currentName, onSubmit });
}

export function openImageViewer(id: string, name: string, url: string, readonly: boolean) {
	imageViewer.set({ open: true, id, name, url, readonly });
}

export function openConfirmModal(title: string, message: string, confirmLabel: string, danger: boolean, onConfirm: () => Promise<void>) {
	confirmModal.set({ open: true, title, message, confirmLabel, danger, onConfirm });
}

export function openUploadModal(files: FileList | null = null) {
	uploadModal.set({ open: true, files });
}
