<script lang="ts">
    import type { TableContent } from "./types";
    export let table: TableContent;
    export let page: number;
    export let total_pages: number;
</script>

{#if table.columns.length > 0}
    <div class="tooltip">
        Haz click en las columnas que quieras exportar a CSV, se acumularan
        hasta que presiones descargar!
    </div>

    <div class="table-header">
        <div class="result-count">{table.msg}</div>
        <div class="result-count">{page} de {total_pages}</div>
        <div class="pagination"></div>
    </div>

    <table class="modern-table" id="table-content">
        <thead>
            <tr>
                {#each table.columns as column}
                    <th scope="col" style="cursor:pointer">{column}</th>
                {/each}
            </tr>
        </thead>
        <tbody>
            {#each table.rows as row}
                <tr>
                    {#each row as cell}
                        <td class="csv">
                            {#if cell}
                                {@html cell}
                            {:else}
                                &nbsp;
                            {/if}
                        </td>
                    {/each}
                </tr>
            {/each}
        </tbody>
    </table>
{/if}
