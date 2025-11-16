<script lang="ts">
import type { TableContent } from "./types";

export let table: TableContent;
export const isLoading: boolean = false;
export const isStreaming: boolean = false;
export const streamingCount: number = 0;
</script>

{#if isLoading || isStreaming}
    <div class="status-indicator {isLoading || isStreaming ? 'loading' : 'completed'}">
        <div class="status-text">
            {#if isLoading}
                Buscando data...
            {:else if isStreaming}
                Buscando data... ({streamingCount})
            {/if}
        </div>
    </div>
{:else if table.columns.length > 0}
    <div class="status-indicator completed">
        <div class="status-text">Búsqueda terminada</div>
    </div>
{/if}

{#if table.columns.length > 0}
    <div class="tooltip">
        Haz click en las columnas que quieras exportar a CSV, se acumularan
        hasta que presiones descargar!
    </div>
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
                        <td>
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

<style>
    .modern-table {
        width: 98.5%;
        margin: 2rem;
        background: white;
        border-collapse: collapse;
        background-color: white;
        font-size: 1.1rem;
        overflow: hidden;

        border-radius: var(--my-border-radius);
        border: 1px solid var(--my-light-gray);
        box-shadow: var(--my-popup-shadow);
    }

    .modern-table thead {
        background-color: rgba(0, 0, 0, 0.05);
    }

    .modern-table th {
        text-align: left;
        padding: 12px;
        font-size: 16px;
        background: var(--my-light-gray);
        font-weight: bold;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }

    .modern-table th:hover {
        background-color: #f9fafb;
        color: var(--my-green);
    }

    .modern-table th:active {
        background-color: #93c5fd;
    }

    .modern-table td {
        padding: 12px;
        border-top: 1px solid rgba(0, 0, 0, 0.05);
    }

    .modern-table tr:hover {
        background-color: rgba(0, 0, 0, 0.02);
    }

    .status-indicator {
        padding: 12px 20px;
        margin: 10px 0;
        border-radius: 8px;
        text-align: center;
        font-weight: 600;
        font-size: 0.95rem;
        transition: all 0.3s ease;
    }

    .status-indicator.loading {
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
        box-shadow: 0 0 20px rgba(102, 126, 234, 0.4);
        animation: glow-pulse 2s ease-in-out infinite alternate;
    }

    .status-indicator.completed {
        background: linear-gradient(135deg, #56ab2f 0%, #a8e6cf 100%);
        color: white;
        box-shadow: 0 0 15px rgba(86, 171, 47, 0.3);
    }

    .status-text {
        text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    }

    @keyframes glow-pulse {
        0% {
            box-shadow: 0 0 20px rgba(102, 126, 234, 0.4);
            transform: scale(1);
        }
        100% {
            box-shadow: 0 0 30px rgba(102, 126, 234, 0.8);
            transform: scale(1.02);
        }
    }

    /* Responsive design for status indicator */
    @media (max-width: 768px) {
        .status-indicator {
            font-size: 0.9rem;
            padding: 10px 16px;
        }
    }
</style>
