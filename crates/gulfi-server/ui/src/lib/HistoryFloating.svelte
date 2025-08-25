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

function handleKeydown(event: KeyboardEvent) {
  const isToggleHotkey = event.ctrlKey && event.key === "h";

  if (isToggleHotkey) {
    event.preventDefault();
    visible = !visible;

    if (visible) {
      fetchHistory();
    }
    return;
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
	
.history-list {
	display: flex;
	flex-direction: column;
	gap: 16px;
	padding: 16px;
	max-width: 900px;
	margin: 0 auto;
	list-style-type: none;
}

.history-overlay {
	position: fixed;
	top: 0;
	left: 0;
	width: 100%;
	height: 100%;

	background-color: rgba(0, 0, 0, 0.5);
	display: flex;
	justify-content: center;
	align-items: center;
	z-index: 1000;
}

.history-window {
	background-color: var(--my-bg);
	border-radius: 8px;
	box-shadow:
		0 4px 20px rgba(0, 0, 0, 0.2),
		inset 0 0 4px 4px rgba(0, 0, 0, 0.1);
	width: 500px;
	height: 500px;
	max-width: 90%;
	max-height: 80vh;
	overflow: hidden;
	display: flex;
	flex-direction: column;
}

.history-header {
	padding: 16px;
	background-color: var(--my-dark-gray);
	border-bottom: 1px solid #ddd;
	display: flex;
	justify-content: space-between;
	align-items: center;
}

.history-title {
	font-weight: bold;
	color: white;
	font-size: 18px;
	margin: 0;
}

.history-body {
	flex: 1;
	overflow-y: auto;
	padding: 0;
}

.history-item {
	padding: 12px 16px;
	border-bottom: 1px solid #eee;
	display: flex;
	justify-content: space-between;
	align-items: center;
	cursor: pointer;
}

.history-item:hover {
	background-color: var(--my-green);
}

.query-text {
	flex: 1;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.empty-history {
	padding: 20px;
	text-align: center;
	color: #666;
	max-width: 500px;
	margin: auto;
	background: var(--my-light-gray);
	border-radius: 10px;
	position: absolute;
	top: 50%;
	left: 50%;
	transform: translate(-50%, -50%);
}

</style>
