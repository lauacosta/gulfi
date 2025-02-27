<script lang="ts">
    import { onMount } from "svelte";

    type Resultados = {
        id: number;
        nombre: string;
        data: string;
        fecha: string;
        busquedas: string[];
    };

    type Favoritos = {
        favoritos: Resultados[];
    };

    export let favoritos: Favoritos = { favoritos: [] };

    const descargarCSVFavoritos = (data: string, nombre: string) => {
        const blob = new Blob([data], { type: "text/csv" });
        const url = URL.createObjectURL(blob);
        const link = document.createElement("a");
        link.href = url;
        link.download = `${nombre}.csv`;
        link.click();
    };
    const borrarFavorito = async (nombre: string) => {
        try {
            const response = await fetch(
                `/api/favoritos?nombre=${encodeURIComponent(nombre)}`,
                {
                    method: "DELETE",
                },
            );

            if (response.ok) {
                favoritos.favoritos = favoritos.favoritos.filter(
                    (fav) => fav.nombre !== nombre,
                );
            } else {
                console.error(`Fallo al borrar favorito: ${nombre}`);
            }
        } catch (error) {
            console.error(`Error eliminando favorito: ${error}`);
        }
    };
    const fetchFavoritos = async () => {
        try {
            const response = await fetch("/api/favoritos");

            if (response.ok) {
                const data: Favoritos = await response.json();
                favoritos = data;
            } else {
                console.error(
                    "Fallo al hacer fetch en favoritos:",
                    response.statusText,
                );
            }
        } catch (error) {
            console.error("Error al hacer fetchi en favoritos:", error);
        }
    };

    onMount(() => {
        fetchFavoritos();
    });
</script>

<main>
    <div class="card-container">
        {#each favoritos.favoritos as fav (fav.id)}
            <div class="card" id="card-{fav.nombre}">
                <h3 class="card-title">{fav.nombre}</h3>
                <div class="card-date">{fav.fecha}</div>
                <div class="tag-list">
                    {#each fav.busquedas as busqueda}
                        <span class="tag">#{busqueda}</span>
                    {/each}
                </div>
                <div class="card-content wrap-text">
                    {fav.data.slice(0, 300)}...
                </div>
                <button
                    on:click={() => borrarFavorito(fav.nombre)}
                    class="delete-button"
                    aria-label="Borrar de favoritos"
                    title="Borrar de favoritos"
                >
                    Borrar
                </button>
                <button
                    class="search-button"
                    on:click={() => descargarCSVFavoritos(fav.data, fav.nombre)}
                    aria-label="Descargar"
                    title="Descargar"
                >
                    Descargar
                </button>
            </div>
        {/each}
    </div>
</main>
