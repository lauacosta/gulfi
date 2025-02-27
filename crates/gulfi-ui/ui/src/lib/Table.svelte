<script lang="ts">
    export let table: {
        msg: string;
        columns: string[];
        rows: string[][];
    };
</script>

<div class="table-header">
    <div class="result-count">{table.msg}</div>
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
