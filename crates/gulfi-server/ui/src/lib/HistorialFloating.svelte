<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { selectedDocument } from "../stores";

    type HistorialResponse = {
        id: number;
        query: string;
    };

    const apiUrl = import.meta.env.VITE_API_URL;

    let historialItems: HistorialResponse[] = $state([]);
    let visible = $state(false);

    async function fetchHistorial() {
        try {
            let endpoint = `${apiUrl}/api/${$selectedDocument}/historial`;
            const response = await fetch(endpoint);

            if (response.ok) {
                historialItems = await response.json();
            } else {
                console.error("Failed to fetch historial");
            }
        } catch (error) {
            console.error("Error fetching historial:", error);
        }
    }

    async function deleteHistorialItem(id: number, queryText: string) {
        try {
            let delete_url = 
                `${apiUrl}/api/${$selectedDocument}/historial?query=${encodeURIComponent(queryText)}`;

            const deleteResponse = await fetch(
                delete_url,
                {
                    method: "DELETE",
                },
            );

            if (deleteResponse.ok) {
                historialItems = historialItems.filter(
                    (item) => item.id !== id,
                );
            } else {
                throw Error("Error al eliminar el elemento.");
            }
        } catch (error) {
            console.error(
                "Ha ocurrido un error al intentar eliminar el elemento.",
            );
        }
    }

    function handleKeydown(event) {
        if (event.ctrlKey && event.key === "h") {
            event.preventDefault();
            visible = true;
            fetchHistorial();
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
        class="historial-overlay"
        role="button"
        tabindex="0"
        onclick={() => (visible = false)}
        onkeydown={(e) => e.key === "Enter" && (visible = false)}
        aria-label="Close overlay"
    >
        <div class="historial-window">
            <div class="historial-header">
                <h2 class="historial-title">Historial de búsquedas</h2>
            </div>
            <div class="historial-body">
                {#if historialItems.length > 0}
                    <ul class="historial-list">
                        {#each historialItems as item (item.id)}
                            <li class="historial-item">
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
                                        deleteHistorialItem(
                                            item.id,
                                            item.query,
                                        );
                                    }}
                                    title="Eliminar de historial"
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
                    <div class="empty-historial">
                        No hay búsquedas en el historial
                    </div>
                {/if}
            </div>
        </div>
    </div>
{/if}

<style>
</style>
