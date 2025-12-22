<script lang="ts">
import { onMount } from "svelte";
import HistoryList from "../lib/HistoryList.svelte";
import type { History } from "../lib/types";
import { selectedDocument } from "../stores";

const apiUrl = import.meta.env.VITE_API_URL;

let history: History[] = [];

// WARN: Siempre se va a ver unicamente el query con su ultimo metodo de busqueda
const fetchHistory = async () => {
	try {
		const fetchstr = `${apiUrl}/api/${$selectedDocument}/history-full`;
		console.log(fetchstr);
		const response = await fetch(fetchstr);

		if (response.ok) {
			const data: History[] = await response.json();
			history = data;
		} else {
			console.error("Fallo al traer el history:", response.statusText);
		}
	} catch (error) {
		console.error("Fallo al traer el history:", error);
	}
};

onMount(() => {
	fetchHistory();
});
</script>

<HistoryList items={history} />
