<script lang="ts">
import { onDestroy, onMount } from "svelte";
import { selectedDocument } from "../stores";

type HistoryResponse = {
	id: number;
	query: string;
};

const apiUrl = import.meta.env.VITE_API_URL;

let historyItems: HistoryResponse[] = $state([]);
let visible = $state(false);

async function fetchHistory() {
	try {
		const endpoint = `${apiUrl}/api/${$selectedDocument}/history`;
		const response = await fetch(endpoint);

		if (response.ok) {
			historyItems = await response.json();
		} else {
			console.error("Failed to fetch history");
		}
	} catch (error) {
		console.error("Error fetching history:", error);
	}
}

async function deleteHistoryItem(id: number, queryText: string) {
	try {
		const delete_url = `${apiUrl}/api/${$selectedDocument}/history?query=${encodeURIComponent(queryText)}`;

		const deleteResponse = await fetch(delete_url, {
			method: "DELETE",
		});

		if (deleteResponse.ok) {
			historyItems = historyItems.filter((item) => item.id !== id);
		} else {
			throw Error("Error al eliminar el elemento.");
		}
	} catch (error) {
		console.error("Ha ocurrido un error al intentar eliminar el elemento.");
	}
}

function handleKeydown(event) {
	if (event.ctrlKey && event.key === "h") {
		event.preventDefault();
		visible = true;
		fetchHistory();
	}

	if (event.key === "Escape" && visible) {
		visible = false;
	}
}

function handleItemClick(query: string) {
	const query_trimmed = query.trim();
	const event = new CustomEvent("select-query", {
		detail: { query_trimmed },
		bubbles: true,
	});
	document.dispatchEvent(event);

	visible = false;
}

onMount(() => {
	window.addEventListener("keydown", handleKeydown);
});

onDestroy(() => {
	window.removeEventListener("keydown", handleKeydown);
});
</script>

{#if visible}
    <div
        class="history-overlay"
        role="button"
        tabindex="0"
        onclick={() => (visible = false)}
        onkeydown={(e) => e.key === "Enter" && (visible = false)}
        aria-label="Close overlay"
    >
        <div class="history-window">
            <div class="history-header">
                <h2 class="history-title">History de búsquedas</h2>
            </div>
            <div class="history-body">
                {#if historyItems.length > 0}
                    <ul class="history-list">
                        {#each historyItems as item (item.id)}
                            <li class="history-item">
                                <span
                                    class="query-text"
                                    onclick={() => handleItemClick(item.query)}
                                >
                                    {item.query}
                                </span>
                                <button
                                    aria-label="borrar elemento"
                                    class="delete-btn"
                                    onclick={(e) => {
                                        e.stopPropagation();
                                        deleteHistoryItem(
                                            item.id,
                                            item.query,
                                        );
                                    }}
                                    title="Eliminar de history"
                                >
                                    <svg
                                        width="20"
                                        height="20"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        ><polyline points="3 6 5 6 21 6"
                                        ></polyline><path
                                            d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                                        ></path></svg
                                    >
                                </button>
                            </li>
                        {/each}
                    </ul>
                {:else}
                    <div class="empty-history">
                        No hay búsquedas en el history
                    </div>
                {/if}
            </div>
        </div>
    </div>
{/if}

<style>
</style>
