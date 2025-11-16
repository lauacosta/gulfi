import { writable } from "svelte/store";

const saved = localStorage.getItem("selectedDocument");
export const selectedDocument = writable<string | null>(saved);

selectedDocument.subscribe((value) => {
	if (value) {
		localStorage.setItem("selectedDocument", value);
	}
});
