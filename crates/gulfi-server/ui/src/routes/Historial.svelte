<script lang="ts">
    import { onMount } from "svelte";
    import type { Historial } from "../lib/types";
    import HistorialList from "../lib/HistorialList.svelte";

    const apiUrl = import.meta.env.VITE_API_URL;

    let historial: Historial[] = $state([]);

    // WARN: Siempre se va a ver unicamente el query con su ultimo metodo de busqueda
    const fetchHistorial = async () => {
        try {
            const response = await fetch(`${apiUrl}/api/historial-full`);

            if (response.ok) {
                const data: Historial[] = await response.json();
                historial = data;
                console.log(data);
            } else {
                console.error(
                    "Fallo al hacer fetch en en historial:",
                    response.statusText,
                );
            }
        } catch (error) {
            console.error("Error al hacer fetch en en historial:", error);
        }
    };

    onMount(() => {
        fetchHistorial();
    });
</script>

<HistorialList items={historial} />
