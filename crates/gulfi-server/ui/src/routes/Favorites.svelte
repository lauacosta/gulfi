<script lang="ts">
    import { onMount } from "svelte";
    import { getBgColor } from "../lib/utils";
    import Empty from "../lib/Empty.svelte";
    import { selectedDocument } from "../stores";
    const apiUrl = import.meta.env.VITE_API_URL;

    type Resultados = {
        id: number;
        nombre: string;
        data: string;
        fecha: string;
        busquedas: [string, string][];
    };

    type Favorites = {
        favorites: Resultados[];
    };

    export let favorites: Favorites = { favorites: [] };

    const descargarCSVFavorites = (data: string, nombre: string) => {
        data = data.trim();
        data = data.replace(/^(\[)+|(\])+/g, "");
        const items = data.split(",").map((item) => item.trim());
        const csvString = items.join("\n");

        const blob = new Blob([csvString], { type: "text/csv" });
        const url = URL.createObjectURL(blob);
        const link = document.createElement("a");
        link.href = url;
        link.download = `${nombre}.csv`;
        link.click();
        URL.revokeObjectURL(url);
    };

    const borrarFavorito = async (nombre: string) => {
        try {
            const response = await fetch(
                `${apiUrl}/api/${$selectedDocument}/favorites?nombre=${encodeURIComponent(nombre)}`,
                {
                    method: "DELETE",
                },
            );

            if (response.ok) {
                favorites.favorites = favorites.favorites.filter(
                    (fav) => fav.nombre !== nombre,
                );
            } else {
                console.error(`Fallo al borrar favorito: ${nombre}`);
            }
        } catch (error) {
            console.error(`Error eliminando favorito: ${error}`);
        }
    };

    const fetchFavorites = async () => {
        try {
            const response = await fetch(
                `${apiUrl}/api/${$selectedDocument}/favorites`,
            );

            if (response.ok) {
                const data: Favorites = await response.json();
                favorites = data;
                console.log(data);
            } else {
                console.error(
                    "Fallo al hacer fetch en favorites:",
                    response.statusText,
                );
            }
        } catch (error) {
            console.error("Error al hacer fetch en favorites:", error);
        }
    };

    onMount(() => {
        fetchFavorites();
    });
</script>

<main>
    {#if favorites.favorites.length > 0}
        <div class="legend">
            <div class="legend-title">Referencia</div>
            <div class="legend-item">
                <span class="color-sample fts"></span>
                <span class="legend-text">Full Text Search </span>
            </div>
            <div class="legend-item">
                <span class="color-sample reciprocal-rank-fusion"></span>
                <span class="legend-text">Reciprocal Rank Fusion</span>
            </div>

            <div class="legend-item">
                <span class="color-sample semantic"></span>
                <span class="legend-text">Semantica</span>
            </div>
        </div>

        <div class="list-header">
            <h2 class="list-title">Favorites</h2>
        </div>
        <div class="card-container">
            {#each favorites.favorites as fav (fav.id)}
                <div class="card" id="card-{fav.nombre}">
                    <h3 class="card-title">{fav.nombre}</h3>
                    <div class="card-date">{fav.fecha}</div>
                    <div class="tag-list">
                        {#each fav.busquedas as busqueda}
                            <span
                                class="tag {getBgColor(busqueda[1])}"
                                data-tooltip={`La búsqueda fue con: ${busqueda[1]}`}
                                >#{busqueda[0]}</span
                            >
                        {/each}
                    </div>
                    <div class="card-content wrap-text">
                        {fav.data.slice(0, 300)}...
                    </div>
                    <button
                        onclick={() => borrarFavorito(fav.nombre)}
                        class="btn delete-button"
                        aria-label="Borrar de favorites"
                        title="Borrar de favorites"
                    >
                        Borrar
                    </button>
                    <button
                        class="btn search-button"
                        onclick={() =>
                            descargarCSVFavorites(fav.data, fav.nombre)}
                        aria-label="Descargar"
                        title="Descargar"
                    >
                        Descargar
                    </button>
                </div>
            {/each}
        </div>
    {:else}
        <Empty
            titulo="No tienes ningún favorito"
            motivo="Parece que aún no has agregado ningún favorito. Guarda tus
                búsquedas más importantes para acceder a ellas fácilmente en el
                futuro."
            solucion="Empieza explorando y haz clic en el botón 'Guardar como favorito' para añadir uno."
        />
    {/if}
</main>
