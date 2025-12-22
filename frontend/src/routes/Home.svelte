<script lang="ts">
import { onDestroy, onMount, tick } from "svelte";
import { writable } from "svelte/store";
import HistoryFloating from "../lib/HistoryFloating.svelte";
import Table from "../lib/Table.svelte";
import type { SearchStrategy, ServerError, TableContent } from "../lib/types";
import { inputPopUp, renderSearchError } from "../lib/utils";
import { selectedDocument } from "../stores";

const apiUrl = import.meta.env.VITE_API_URL;

let searchState = $state({
	query: "",
	strategy: "Fts" as SearchStrategy,
	sexo: "U",
	edad_min: 0,
	edad_max: 100,
	k: 1000,
	peso_fts: 50,
	peso_semantic: 50,
	isLoading: false,
	isStreaming: false,
	error: null as ServerError | null,
});

const showOcultables = $derived(searchState.strategy !== "Fts");
const showBalanceSlider = $derived(
	searchState.strategy === "ReciprocalRankFusion",
);
const sliderValue = $derived.by(() => searchState.peso_fts);

const tableContent = writable<TableContent>({ msg: "", columns: [], rows: [] });
let downloadState = $state({
	data: [] as string[],
	headers: [] as string[],
	count: 0,
	weight: "0 KB",
	disabled: true,
});

let streamingResults = $state<string[][]>([]);
let streamingColumns = $state<string[]>([]);
let eventSource: EventSource | null = null;

const shortcuts = [
	{
		key: "b",
		ctrl: true,
		action: () => document.getElementById("search-input")?.focus(),
	},
	{ key: "s", ctrl: true, shift: true, action: downloadCSV },
	{ key: "f", ctrl: true, shift: true, action: saveFavorite },
];

onMount(() => {
	loadUrlParams();
	setupKeyboardShortcuts();
});

onDestroy(() => {
	eventSource?.close();
	shortcuts.forEach(({ key, ctrl, shift }) => {
		document.removeEventListener("keydown", handleKeydown);
	});
});

function loadUrlParams() {
	const params = new URLSearchParams(window.location.search);
	if (!params.toString()) return;

	searchState = {
		...searchState,
		query: params.get("query") || "",
		strategy: (params.get("strategy") as SearchStrategy) || "Fts",
		sexo: params.get("sexo") || "U",
		edad_min: Number(params.get("edad_min")) || 0,
		edad_max: Number(params.get("edad_max")) || 100,
		peso_fts: Number(params.get("peso_fts")) || 50,
		peso_semantic: Number(params.get("peso_semantic")) || 50,
		k: Number(params.get("neighbors")) || 1000,
	};
}

function setupKeyboardShortcuts() {
	document.addEventListener("keydown", handleKeydown, { capture: true });
	document.addEventListener("select-query", handleQuerySelect);
}

function handleKeydown(event: KeyboardEvent) {
	const shortcut = shortcuts.find(
		(s) =>
			s.key.toLowerCase() === event.key.toLowerCase() &&
			!!s.ctrl === event.ctrlKey &&
			!!s.shift === event.shiftKey,
	);

	if (shortcut) {
		event.preventDefault();
		event.stopImmediatePropagation();
		shortcut.action();
	}
}

function updateSliderValues(value: number) {
	searchState.peso_fts = value;
	searchState.peso_semantic = 100 - value;
}

async function handleSearch(event: SubmitEvent) {
	event.preventDefault();
	searchState.isLoading = true;
	searchState.error = null;

	const formData = new FormData(event.target as HTMLFormElement);
	const params = new URLSearchParams();

	for (const [key, value] of formData) {
		params.append(key, value.toString());
	}

	try {
		const response = await fetch(`${apiUrl}/api/search?${params}`);

		if (!response.ok) {
			searchState.error = await response.json();
			return;
		}

		const table: TableContent = await response.json();
		tableContent.set(table);

		await tick();
		initializeTableFeatures();
	} catch (err) {
		searchState.error = {
			err: "Error desconocido",
			date: new Date().toISOString(),
		};
	} finally {
		searchState.isLoading = false;
	}
}

async function handleStreamingSearch(event: SubmitEvent) {
	event.preventDefault();

	if (eventSource) {
		eventSource.close();
		eventSource = null;
	}

	searchState.isStreaming = true;
	searchState.error = null;
	streamingResults = [];
	streamingColumns = [];

	const formData = new FormData(event.target as HTMLFormElement);
	const params = new URLSearchParams();

	for (const [key, value] of formData) {
		params.append(key, value.toString());
	}
	params.append("batch_size", "10");

	try {
		const sseUrl = `${apiUrl}/api/search_stream?${params}`;
		eventSource = new EventSource(sseUrl);

		eventSource.onopen = () => {
			console.log("SSE connection opened");
		};

		eventSource.onmessage = (event) => {
			try {
				const message = JSON.parse(event.data);

				if (message.type === "metadata") {
					streamingColumns = message.columns;
					tableContent.set({
						msg: `Recibiendo resultados...`,
						columns: streamingColumns,
						rows: [],
					});
				} else if (message.type === "rows" && message.data) {
					streamingResults = [...streamingResults, ...message.data];
					tableContent.set({
						msg: `Recibiendo resultados (${streamingResults.length})`,
						columns: streamingColumns,
						rows: streamingResults,
					});
				} else if (message.type === "complete") {
					finishStreaming();
				}
			} catch (parseError) {
				console.error("Error parsing SSE message:", parseError);
			}
		};

		eventSource.onerror = (event) => {
			console.error("SSE error:", event);

			if (eventSource?.readyState === EventSource.CLOSED) {
				console.log("SSE connection closed");
				finishStreaming();
			} else {
				searchState.error = {
					err: "Error en la conexión SSE",
					date: new Date().toISOString(),
				};
				finishStreaming();
			}
		};
	} catch (err) {
		console.error("Error creating SSE connection:", err);
		searchState.isStreaming = false;
		searchState.error = {
			err: "Error al conectar con el servidor",
			date: new Date().toISOString(),
		};
	}
}

function finishStreaming() {
	searchState.isStreaming = false;

	if (eventSource) {
		eventSource.close();
		eventSource = null;
	}

	tableContent.set({
		msg: `Found ${streamingResults.length} results`,
		columns: streamingColumns,
		rows: streamingResults,
	});

	tick().then(initializeTableFeatures);
}

function cancelStreaming() {
	if (eventSource) {
		eventSource.close();
		eventSource = null;
	}
	searchState.isStreaming = false;
}

function initializeTableFeatures() {
	initPagination();
	setupColumnSelection();
}

function initPagination() {
	const table = document.querySelector(".modern-table");
	if (!table) return;

	const itemsPerPage = 10;
	let currentPage = 0;
	const rows: Element[] = Array.from(table.querySelectorAll("tbody tr"));
	const totalPages = Math.ceil(rows.length / itemsPerPage);

	function showPage(page: number) {
		const start = page * itemsPerPage;
		const end = start + itemsPerPage;

		rows.forEach((row, i) => {
			row.style.display = i >= start && i < end ? "" : "none";
		});

		const pageInfo = document.querySelector(".page-info");
		if (pageInfo) pageInfo.textContent = `Página ${page + 1} de ${totalPages}`;
	}

	const container = document.querySelector(".pagination");
	if (!container) return;

	const buttons = [
		{ text: "<<", action: () => showPage((currentPage = 0)) },
		{ text: "<", action: () => currentPage > 0 && showPage(--currentPage) },
		{
			text: ">",
			action: () => currentPage < totalPages - 1 && showPage(++currentPage),
		},
		{ text: ">>", action: () => showPage((currentPage = totalPages - 1)) },
	];

	container.innerHTML = "";
	buttons.forEach(({ text, action }, i) => {
		const btn = document.createElement("button");
		btn.textContent = text;
		btn.onclick = (e) => {
			e.preventDefault();
			action();
		};
		container.appendChild(btn);

		if (i === 1) {
			const info = document.createElement("span");
			info.className = "page-info";
			container.appendChild(info);
		}
	});

	showPage(0);
}

function setupColumnSelection() {
	const headers = document.querySelectorAll("#table-content thead th");
	const table = document.getElementById("table-content");
	if (!table) return;

	headers.forEach((header, index) => {
		header.addEventListener("mousedown", () => {
			const columnData = [
				header.textContent?.trim() || "",
				...Array.from(table.querySelectorAll("tbody tr")).map((row) => {
					const cell = row.children[index] as HTMLElement;
					return cell?.textContent?.trim().replace(/\n/g, "") || "";
				}),
			];

			downloadState.headers = [header.textContent?.trim() || ""];
			downloadState.data = columnData.slice(1);
			downloadState.count = downloadState.data.length;
			downloadState.weight = `${(new Blob([JSON.stringify(downloadState.data)]).size / 1024).toFixed(2)} KB`;
			downloadState.disabled = false;
		});
	});
}

function resetDownload() {
	downloadState = {
		data: [],
		headers: [],
		count: 0,
		weight: "0 KB",
		disabled: true,
	};
}

function downloadCSV() {
	if (!downloadState.data.length) return;

	const csv = downloadState.data.join("\n");
	const blob = new Blob([csv], { type: "text/csv" });
	const url = URL.createObjectURL(blob);

	const a = document.createElement("a");
	a.href = url;
	a.download = `busqueda_${searchState.query.replace(/\s+/g, "_")}_${new Date().toLocaleDateString("es-AR").replace(/\//g, "")}.csv`;
	a.click();

	URL.revokeObjectURL(url);
	resetDownload();
}

async function saveFavorite() {
	if (!downloadState.data.length) return;

	const name = await inputPopUp("Ingresa un nombre para guardarlo");
	if (!name?.trim()) return;

	const sanitizedName = name.replace(/[^a-zA-Z0-9_\-\s]/g, "");

	try {
		const response = await fetch(
			`${apiUrl}/api/${$selectedDocument}/favorites`,
			{
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({
					nombre: sanitizedName,
					data: downloadState.data,
					busquedas: [
						{ query: searchState.query, strategy: searchState.strategy },
					],
				}),
			},
		);

		if (!response.ok) throw new Error("Error al guardar");
	} catch (err) {
		console.error("Error:", err);
	}
}

function handleQuerySelect(event: CustomEvent<{ query_trimmed: string }>) {
	const input = document.getElementById("search-input") as HTMLInputElement;
	if (input) {
		input.value = event.detail.query_trimmed;
		input.focus();
	}
}
</script>

<main class="main-content">
    <HistoryFloating />
    
    <div class="legend">
        <div class="legend-title">Atajos</div>
        {#each [
            { key: "Ctrl+b", desc: "Buscar" },
            { key: "Ctrl+h", desc: "Abrir History" },
            { key: "Ctrl+Shift+s", desc: "Descargar CSV" },
            { key: "Ctrl+Shift+f", desc: "Añadir a Favorites" }
        ] as shortcut}
            <div class="legend-item">
                <span class="kbssample">{shortcut.key}</span>
                <span class="legend-text">{shortcut.desc}</span>
            </div>
        {/each}
    </div>

    <div class="form-container">
        <form onsubmit={handleStreamingSearch}>
            <div class="config-options">
                <div class="search-group">
                    <label for="strategy">Método de Búsqueda:</label>
                    <select id="strategy" name="strategy" bind:value={searchState.strategy}>
                        <option value="Fts">Full Text Search</option>
                        <option value="Semantic">Semántica</option>
                        <option value="ReciprocalRankFusion">Reciprocal Rank Fusion</option>
                    </select>
                </div>

                 {#if showOcultables}
                    <div class="search-group">
                        <label for="k">N° de Vecinos:</label>
                        <input type="number" id="k" name="k" min="1" max="4096" bind:value={searchState.k} />
                    </div>
                {:else}
                    <input type="hidden" name="k" value={searchState.k} />
                {/if}

                {#if showBalanceSlider}
                    <div class="search-group">
                        <label for="balance">Pesos:</label>
                        <input 
                            type="range" 
                            id="balance" 
                            min="0" 
                            max="100" 
                            bind:value={searchState.peso_fts}
                            oninput={(e) => updateSliderValues(+e.target.value)}
                        />
                        <p>FTS: {searchState.peso_fts} | Semantic: {searchState.peso_semantic}</p>
                    </div>
                {/if}

                <!-- Hidden fields -->
                <input type="hidden" name="document" value={$selectedDocument} />
                <input type="hidden" name="peso_fts" value={searchState.peso_fts} />
                <input type="hidden" name="peso_semantic" value={searchState.peso_semantic} />
            </div>

            <div class="search-group search-bar full-width">
                <label for="search-input">Búsqueda:</label>
                <input
                    type="text"
                    id="search-input"
                    name="query"
                    placeholder="Ingresa tu busqueda..."
                    bind:value={searchState.query}
                    required
                />
            </div>

            {#if searchState.error}
                <div class="error-box">
                    <p>{renderSearchError(searchState.error)}</p>
                </div>
            {/if}

            <div class="button-container">
                {#if searchState.isStreaming}
                    <button type="button" class="btn btn-cancel" onclick={cancelStreaming}>
                        Cancelar ({streamingResults.length} resultados)
                    </button>
                {:else}
                    <button type="submit" class="btn search-button" disabled={searchState.isLoading}>
                        {searchState.isLoading ? 'Buscando...' : 'Buscar'}
                    </button>
                {/if}

                <button 
                    type="button" 
                    class="btn search-button" 
                    disabled={downloadState.disabled}
                    onclick={downloadCSV}
                >
                    Descargar {downloadState.count} elementos ({downloadState.weight})
                </button>

                <button type="button" class="btn btn-icon" disabled={downloadState.disabled} onclick={resetDownload}>
                    <!-- Reset icon SVG -->
                </button>

                <button type="button" class="btn btn-icon" disabled={downloadState.disabled} onclick={saveFavorite}>
                    <!-- Star icon SVG -->
                </button>
            </div>
        </form>
    </div>

    <div>
        <Table 
        table={$tableContent} 
        isLoading={searchState.isLoading}
        isStreaming={searchState.isStreaming}
        streamingCount={streamingResults.length}
        />
    </div>
</main>

<style>
    .kbssample {
        background-color: var(--my-dark-gray);
        border: 1px solid #ddd;
        border-radius: 5px;
        color: white;
        font-weight: bold;
        padding: 2px 5px;
        margin-right: 10px;
        width: 100px;
        min-width: 80px;
        display: inline-block;
        text-align: center;
    }

    .form-container {
        max-width: 100%;
        margin: 1rem 2rem auto;
        background-color: white;
        padding: 20px;
        border: 1px solid var(--my-light-gray);
        border-radius: var(--my-border-radius);
        box-shadow: var(--my-shadow);
    }

    .form-container input[type="text"],
    .form-container input[type="number"],
    .form-container select {
        width: 100%;
        padding: 12px;
        border-radius: 8px;
        border: 1px solid rgba(0, 0, 0, 0.1);
        background-color: rgba(0, 0, 0, 0.02);
        font-size: 15px;
        transition: all 0.2s ease;
    }

    .form-container input[type="text"]:focus,
    .form-container input[type="number"]:focus,
    .form-container select:focus {
        outline: none;
        border-color: var(--my-green);
        box-shadow: var(--my-shadow);
    }

    .config-options {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
        gap: 1rem;
    }

    @media (max-width: 768px) {
        .config-options {
            grid-template-columns: 1fr;
        }
    }

    .search-group label {
        display: block;
        margin-bottom: 8px;
        font-size: 18px;
        font-weight: 500;
        color: var(--my-mid-gray);
    }

    .search-group {
        position: relative;
        flex-direction: column;
        padding-top: 15px;
    }

    .search-group [type="range"] {
        width: 100%;
    }

    .search-bar {
        margin-top: 2rem;
        font-size: 1.25rem;
        padding: 1rem;
        width: 100%;
        box-sizing: border-box;
    }

    .search-bar.full-width {
        width: 100%;
    }

    .error-box {
        box-shadow: var(--my-popup-shadow);
        background-color: #fee2e2;
        color: #b91c1c;
        padding: 12px;
        border-radius: 6px;
        margin-bottom: 16px;
        display: flex;
        align-items: center;
    }

    .button-container {
        display: flex;
        gap: 0.5rem;
        margin-top: 0.5rem;
    }

    .btn-cancel {
        background-color: #dc3545;
        color: white;
    }

    .btn-cancel:hover {
        background-color: #c82333;
    }

    @keyframes pulse {
        0% {
            opacity: 1;
            transform: scale(1);
        }
        50% {
            opacity: 0.5;
            transform: scale(1.2);
        }
        100% {
            opacity: 1;
            transform: scale(1);
        }
    }
</style>
