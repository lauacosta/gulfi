<script lang="ts">
    import { onMount } from "svelte";

    type Field = {
        name: string;
        unique: boolean;
        vec_input: boolean;
    };

    type Document = {
        name: string;
        fields: Field[];
    };

    const apiUrl = import.meta.env.VITE_API_URL;

    let documents: Document[] = $state([]);

    async function fetch_documents(): Promise<Document[]> {
        const response = await fetch(`${apiUrl}/api/documents`);
        return await response.json();
    }

    onMount(async () => {
        documents = await fetch_documents();

        console.table(documents);
    });
</script>

<div class="search-group">
    <label for="document">Documento:</label>
    <select id="document" name="document" class="search-type">
        {#each documents as doc}
            <option value={doc.name}>{doc.name}</option>
        {/each}
    </select>
</div>
