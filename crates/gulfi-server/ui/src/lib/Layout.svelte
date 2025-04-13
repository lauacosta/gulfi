<script lang="ts">
    import { onMount } from "svelte";
    import { selectedDocument } from "../stores";
    import type { Document } from "./types";

    let options: string[] = $state([]);

    const apiUrl = import.meta.env.VITE_API_URL;
    async function fetch_documents(): Promise<Document[]> {
        const response = await fetch(`${apiUrl}/api/documents`);
        return await response.json();
    }

    onMount(async () => {
        const response = await fetch_documents();
        options = response.map((doc) => doc.name);

        if ($selectedDocument === null && options.length > 0) {
            selectedDocument.set(options[0]);
        }
    });
</script>

<div class="top-bar">
    <label>
        Documento:
        <select bind:value={$selectedDocument}>
            {#each options as option}
                <option value={option}>{option}</option>
            {/each}
        </select>
    </label>
</div>

<div class="content-wrapper">
    <slot />
</div>
