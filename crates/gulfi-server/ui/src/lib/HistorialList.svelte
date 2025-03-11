<script lang="ts">
    import { getBgColor } from "../lib/utils";
    import type Historial from "../lib/types";
    export let items: Historial[];
    import Empty from "../lib/Empty.svelte";

    const apiUrl = import.meta.env.VITE_API_URL;

    const strategyLabels = {
        Fts: "Búsqueda de texto completo",
        Semantic: "Búsqueda semántica",
        ReciprocalRankFusion: "ReciprocalRankFusion",
    };

    const sexLabels = {
        U: "Universal",
        M: "Masculino",
        F: "Femenino",
    };

    async function deleteHistorialItem(id: number, queryText: string) {
        try {
            const deleteResponse = await fetch(
                `${apiUrl}/api/historial?query=${encodeURIComponent(queryText)}`,
                {
                    method: "DELETE",
                },
            );

            if (deleteResponse.ok) {
                items = items.filter((item) => item.id !== id);
            } else {
                throw Error("Error al eliminar el elemento.");
            }
        } catch (error) {
            console.error(
                "Ha ocurrido un error al intentar eliminar el elemento.",
            );
        }
    }

    function buildQueryString(item: Historial) {
        const params = new URLSearchParams();
        params.append("query", item.query);
        params.append("strategy", item.strategy);
        params.append("sexo", item.sexo);
        params.append("edad_min", item.edad_min.toString());
        params.append("edad_max", item.edad_max.toString());
        params.append("peso_fts", item.peso_fts.toString());
        params.append("peso_semantic", item.peso_semantic.toString());
        params.append("neighbors", item.neighbors.toString());

        return `/?${params.toString()}`;
    }

    function getStrategyColor(strategy: string) {
        switch (strategy) {
            case "Fts":
                return "#FF6B6B";
            case "Semantic":
                return "#4ECDC4";
            case "Rrf":
                return "#FFD166";
            default:
                return "#ADB5BD";
        }
    }
</script>

<div>
    {#if items.length > 0}
        <div class="list-header">
            <h2 class="list-title">Historial de búsquedas</h2>
        </div>
    {/if}

    <div class="historial-card-container">
        {#if items.length > 0}
            {#each items as item (item.id)}
                <div class="historial-card">
                    <div class="card-header">
                        <div>
                            <h3 class="query">{item.query}</h3>
                            <div>
                                <span
                                    class="strategy-tag {getBgColor(
                                        item.strategy,
                                    )}"
                                >
                                    {strategyLabels[item.strategy]}
                                </span>
                                <span class="timestamp">{item.fecha}</span>
                            </div>
                        </div>
                    </div>

                    <div class="details-grid">
                        <div class="detail-item">
                            <span class="detail-label">Sexo</span>
                            <span class="detail-value"
                                >{sexLabels[item.sexo]}</span
                            >
                        </div>

                        <div class="detail-item">
                            <span class="detail-label">Rango de edad</span>
                            <span class="detail-value">
                                {item.edad_min === 0 && item.edad_max === 100
                                    ? "Todas las edades"
                                    : `${item.edad_min} - ${item.edad_max} años`}
                            </span>
                        </div>

                        <div class="detail-item">
                            <span class="detail-label">Nº Vecinos</span>
                            <span class="detail-value">{item.neighbors}</span>
                        </div>

                        <div class="detail-item">
                            <span class="detail-label">Pesos de búsqueda</span>
                            <div class="weights">
                                <p class="weight">
                                    FTS:{(item.peso_fts /
                                        (item.peso_fts + item.peso_semantic)) *
                                        100}%
                                </p>
                                <p class="weight">
                                    Semantica {(item.peso_semantic /
                                        (item.peso_fts + item.peso_semantic)) *
                                        100}%
                                </p>
                            </div>
                        </div>
                    </div>

                    <div class="card-footer">
                        <button
                            onclick={() =>
                                deleteHistorialItem(item.id, item.query)}
                            class="btn delete-button"
                            >Borrar
                        </button>
                        <a
                            href={buildQueryString(item)}
                            class="btn search-button">Buscar</a
                        >
                    </div>
                </div>
            {/each}
        {:else}
            <Empty
                titulo="Tienes el historial vacío"
                motivo="Parece que todavia no has realizado ninguna búsqueda."
                solucion="Empieza explorando y realiza una búsquedan"
            />
        {/if}
    </div>
</div>
