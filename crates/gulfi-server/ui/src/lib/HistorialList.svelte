<script lang="ts">
    import type Historial from "./lib/types";
    export let items: Historial[];

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
    <div class="list-header">
        <h2 class="list-title">Historial de búsquedas</h2>
    </div>

    <div class="historial-card-container">
        {#if items.length > 0}
            {#each items as item (item.id)}
                <div class="historial-card">
                    <div class="card-header">
                        <div>
                            <h3 class="query">{item.query}</h3>
                            <div>
                                <span
                                    class="strategy-tag"
                                    style="background-color: {getStrategyColor(
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
                                <div class="weight-bar">
                                    <div
                                        class="weight-fts"
                                        style="--fts-width: {(item.peso_fts /
                                            (item.peso_fts +
                                                item.peso_semantic)) *
                                            100}%"
                                    ></div>
                                    <div
                                        class="weight-semantic"
                                        style="--semantic-width: {(item.peso_semantic /
                                            (item.peso_fts +
                                                item.peso_semantic)) *
                                            100}%"
                                    ></div>
                                </div>
                                <span class="weight-label">
                                    {Math.round(
                                        (item.peso_fts /
                                            (item.peso_fts +
                                                item.peso_semantic)) *
                                            100,
                                    )}% /
                                    {Math.round(
                                        (item.peso_semantic /
                                            (item.peso_fts +
                                                item.peso_semantic)) *
                                            100,
                                    )}%
                                </span>
                            </div>
                        </div>
                    </div>

                    <div class="card-footer">
                        <a
                            href={buildQueryString(item)}
                            class="btn search-button">Buscar</a
                        >
                        <button
                            onclick={() =>
                                deleteHistorialItem(item.id, item.query)}
                            class="btn delete-button"
                            >Borrar
                        </button>
                    </div>
                </div>
            {/each}
        {:else}
            <div class="empty-state">
                <div class="empty-icon">📜</div>
                <p>No hay búsquedas en el historial</p>
            </div>
        {/if}
    </div>
</div>

<style>
    .historial-list {
        display: flex;
        flex-direction: column;
        gap: 16px;
        padding: 16px;
        max-width: 900px;
        margin: 0 auto;
    }

    .empty-state {
        text-align: center;
        padding: 40px 0;
        color: #6c757d;
    }

    .empty-icon {
        font-size: 48px;
        margin-bottom: 16px;
        opacity: 0.5;
    }

    .historial-card {
        background-color: white;
        border-radius: 8px;
        border: 1px solid #e9ecef;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
        padding: 16px;
        transition:
            transform 0.2s ease,
            box-shadow 0.2s ease;
    }

    .historial-card:hover {
        transform: translateY(-2px);
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    }

    .card-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 12px;
    }

    .query {
        font-size: 18px;
        font-weight: 600;
        margin: 0 0 8px 0;
        word-break: break-word;
    }

    .timestamp {
        color: #6c757d;
        font-size: 12px;
        white-space: nowrap;
        margin-left: 8px;
    }

    .strategy-tag {
        display: inline-block;
        padding: 4px 8px;
        border-radius: 4px;
        font-size: 12px;
        font-weight: 500;
        margin-right: 8px;
        color: white;
    }

    .details-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
        margin-bottom: 16px;
    }

    .detail-item {
        display: flex;
        flex-direction: column;
    }

    .detail-label {
        font-size: 12px;
        color: #6c757d;
        margin-bottom: 4px;
    }

    .detail-value {
        font-size: 14px;
        font-weight: bold;
    }

    .card-footer {
        display: flex;
        margin-top: 1rem;
        justify-content: flex-end;
    }

    .weights {
        display: flex;
        gap: 8px;
        align-items: center;
    }

    .weight-bar {
        height: 8px;
        flex: 1;
        background-color: #e9ecef;
        border-radius: 4px;
        overflow: hidden;
        position: relative;
    }

    .weight-fts,
    .weight-semantic {
        height: 100%;
        position: absolute;
        top: 0;
        left: 0;
    }

    .weight-fts {
        background-color: #ff6b6b;
        width: var(--fts-width);
        z-index: 2;
    }

    .weight-semantic {
        background-color: #4ecdc4;
        width: var(--semantic-width);
        z-index: 1;
    }

    .weight-label {
        font-size: 12px;
        margin-left: 8px;
        white-space: nowrap;
    }

    .list-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 16px;
        padding: 0 16px;
    }

    .list-title {
        font-size: 24px;
        font-weight: 600;
        margin: 0;
    }

    @media (max-width: 768px) {
        .details-grid {
            grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
        }
    }
</style>
