<script lang="ts">
    import { onMount } from "svelte";
    import type { Historial } from "../lib/types";
    import HistorialList from "../lib/HistorialList.svelte";
    import { selectedDocument } from "../stores";

    const apiUrl = import.meta.env.VITE_API_URL;

    let historial: Historial[] = $state([]);

    // WARN: Siempre se va a ver unicamente el query con su ultimo metodo de busqueda
    const fetchHistorial = async () => {
        try {
            let fetchstr = `${apiUrl}/api/${$selectedDocument}/historial-full`;
            console.log(fetchstr);
            const response = await fetch(fetchstr);

            if (response.ok) {
                const data: Historial[] = await response.json();
                historial = data;
            } else {
                console.error(
                    "Fallo al traer el historial:",
                    response.statusText,
                );
            }
        } catch (error) {
            console.error("Fallo al traer el historial:", error);
        }
    };

    onMount(() => {
        fetchHistorial();
    });
</script>

<HistorialList items={historial} />
