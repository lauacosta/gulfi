<script lang="ts">
    import { onMount } from "svelte";
    import Table from "../lib/Table.svelte";
    import HistoryFloating from "../lib/HistoryFloating.svelte";
    import type { ServerError, TableContent } from "../lib/types";
    import type { favoritesResponse } from "../lib/types";
    import type { SearchStrategy } from "../lib/types";
    import { writable } from "svelte/store";
    import { inputPopUp, renderSearchError } from "../lib/utils";
    import { selectedDocument } from "../stores";

    const apiUrl = import.meta.env.VITE_API_URL;

    let heldSearches: favoritesResponse[] = [];
    const tableContent = writable<TableContent>({
        msg: "",
        columns: [],
        rows: [],
    });

    let heldData: Array<string> = [];
    let heldHeaders = [];
    let isHolding = false;
    let itemsCount = $state(0);
    let dataWeight = $state("0 KB");
    let downloadBtnDisabled = $state(true);

    let query = $state("");
    let error: ServerError | null = $state(null);

    let strategy: SearchStrategy = $state("Fts");

    let sexo = $state("U");
    let edad_min = $state(0);
    let edad_max = $state(100);
    let k = $state(1000);
    let sliderValue = $state(50);
    let peso_fts = $state(50);
    let peso_semantic = $state(50);

    let showOcultables = $state(false);
    let showBalanceSlider = $state(false);

    onMount(async () => {
        // await updateHistory();
        checkUrlParams();
        hideElements();
        initKeyboard();
    });

    function checkUrlParams() {
        const params = new URLSearchParams(window.location.search);

        if (!params.toString()) {
            return;
        }

        query = params.get("query") || "";
        strategy = (params.get("strategy") as SearchStrategy) || "Fts";
        sexo = params.get("sexo") || "U";
        edad_min = Number(params.get("edad_min")) || 0;
        edad_max = Number(params.get("edad_max")) || 100;
        peso_fts = Number(params.get("peso_fts")) || 50;
        peso_semantic = Number(params.get("peso_semantic")) || 50;
        k = Number(params.get("neighbors")) || 1000;

        sliderValue = peso_fts;
    }

    function updateSliderValues() {
        peso_fts = sliderValue;
        peso_semantic = 100 - sliderValue;
    }

    function hideElements() {
        if (strategy === "ReciprocalRankFusion") {
            showBalanceSlider = true;
            showOcultables = true;
        } else {
            showBalanceSlider = false;
            showOcultables = strategy !== "Fts";
        }
    }

    function handleStrategyChange() {
        hideElements();
    }

    function initKeyboard() {
        document.addEventListener("keydown", (event) => {
            if (event.ctrlKey && event.key === "b") {
                event.preventDefault();
                document.getElementById("search-input")?.focus();
            }
        });

        window.addEventListener(
            "keydown",
            (event) => {
                if (
                    event.ctrlKey &&
                    event.shiftKey &&
                    event.key.toLowerCase() === "s"
                ) {
                    event.preventDefault();
                    event.stopImmediatePropagation();
                    descargarCSVGlobal();
                    return false;
                }
            },
            { capture: true, passive: false },
        );

        window.addEventListener(
            "keydown",
            (event) => {
                if (
                    event.ctrlKey &&
                    event.shiftKey &&
                    event.key.toLowerCase() === "f"
                ) {
                    event.preventDefault();
                    event.stopImmediatePropagation();
                    saveFavorite();
                    return false;
                }
            },
            { capture: true, passive: false },
        );
    }

    async function handleSearch(event: SubmitEvent) {
        event.preventDefault();

        const formData = new FormData(event.target as HTMLFormElement);
        const searchParams = new URLSearchParams();

        for (const [key, value] of formData.entries()) {
            searchParams.append(key, value.toString());
        }

        try {
            const response = await fetch(
                `${apiUrl}/api/search?${searchParams.toString()}`,
            );

            if (!response.ok) {
                const json = await response.json();
                error = json as ServerError;
                return;
            }

            error = null;
            const table: TableContent = await response.json();

            tableContent.set(table);

            requestAnimationFrame(() => {
                initPagination();
                guardarResultados();
                // updateHistory();
            });
        } catch (error) {
            console.error("Error en la búsqueda:", error);
            error = {
                err: "Error desconocido",
                date: new Date().toISOString(),
            };
        }
    }

    function initPagination() {
        const content = document.querySelector(".modern-table");
        if (!content) return;

        const itemsPerPage = 10;
        let currentPage = 0;
        const items = Array.from(content.getElementsByTagName("tr")).slice(1);
        const totalPages = Math.ceil(items.length / itemsPerPage);

        function showPage(page: number) {
            const startIndex = page * itemsPerPage;
            const endIndex = startIndex + itemsPerPage;

            items.forEach((item, index) => {
                item.style.display =
                    index >= startIndex && index < endIndex ? "" : "none";
            });

            const pageInfo = document.querySelector(".page-info");
            if (pageInfo) {
                pageInfo.textContent = `Página ${page + 1} de ${totalPages}`;
            }
        }

        const paginationContainer = document.querySelector(".pagination");
        if (!paginationContainer) return;

        paginationContainer.innerHTML = "";

        const startButton = document.createElement("button");
        startButton.textContent = "<<";
        startButton.addEventListener("click", (e) => {
            e.preventDefault();
            currentPage = 0;
            showPage(currentPage);
        });

        // Prev button
        const prevButton = document.createElement("button");
        prevButton.textContent = "<";
        prevButton.addEventListener("click", (e) => {
            e.preventDefault();
            if (currentPage > 0) {
                showPage(--currentPage);
            }
        });

        const pageInfo = document.createElement("span");
        pageInfo.classList.add("page-info");
        pageInfo.textContent = `Página ${currentPage + 1} de ${totalPages}`;

        const nextButton = document.createElement("button");
        nextButton.textContent = ">";
        nextButton.addEventListener("click", (e) => {
            e.preventDefault();
            if (currentPage < totalPages - 1) {
                showPage(++currentPage);
            }
        });

        const endButton = document.createElement("button");
        endButton.textContent = ">>";
        endButton.addEventListener("click", (e) => {
            e.preventDefault();
            currentPage = totalPages - 1;
            showPage(currentPage);
        });

        paginationContainer.append(
            startButton,
            prevButton,
            pageInfo,
            nextButton,
            endButton,
        );
        showPage(currentPage);
    }

    function guardarResultados() {
        const table = document.getElementById("table-content");
        if (!table) return;

        const headers =
            table.querySelectorAll<HTMLTableCellElement>("thead th");

        headers.forEach((header, index) => {
            header.addEventListener("mousedown", () => {
                if (!isHolding) {
                    isHolding = true;

                    heldHeaders.push(header.innerText);

                    heldData.push(
                        ...[...table.querySelectorAll("tbody tr")].map(
                            (row) => {
                                const cell = row.children[index] as HTMLElement;
                                return cell
                                    ? cell.innerText.trim().replace(/\n/g, "")
                                    : "";
                            },
                        ),
                    );

                    const weight = calcularPeso(heldData);
                    itemsCount = heldData.length;
                    dataWeight = `${weight} KB`;
                    downloadBtnDisabled = false;

                    if (query.trim()) {
                        heldSearches.push({ query, strategy: strategy });
                    }
                }
            });

            header.addEventListener("mouseup", () => {
                isHolding = false;
            });
        });
    }

    function resetHeldData() {
        heldData = [];
        heldHeaders = [];
        itemsCount = 0;
        dataWeight = "0 KB";
        downloadBtnDisabled = true;
    }

    function descargarCSVGlobal() {
        if (heldData.length === 0) return;

        const csvRows = heldData.map((item) => item.toString());

        const csvString = csvRows.join("\n");
        const blob = new Blob([csvString], { type: "text/csv" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;

        let now = new Date();
        let day = String(now.getDate()).padStart(2, "0");
        let month = String(now.getMonth() + 1).padStart(2, "0");
        let year = now.getFullYear();

        let dateNumber = Number(`${day}${month}${year}`);

        a.download = `busqueda_${query.replace(/\s+/g, "_")}_${dateNumber}.csv`;
        a.click();
        URL.revokeObjectURL(url);

        resetHeldData();
        heldSearches = [];
    }

    async function saveFavorite() {
        if (heldData.length === 0 || heldSearches.length === 0) return;

        const input = await inputPopUp("Ingresa un nombre para guardarlo");

        if (input === null) {
            return;
        }
        const name = input?.replace(/[^a-zA-Z0-9_\-\s]/g, "");

        if (name !== null && name !== "") {
            const data = {
                nombre: name,
                data: heldData,
                busquedas: heldSearches,
            };

            try {
                const response = await fetch(
                    `${apiUrl}/api/${$selectedDocument}/favorites`,
                    {
                        method: "POST",
                        headers: {
                            "Content-Type": "application/json",
                        },
                        body: JSON.stringify(data),
                    },
                );
                if (response.ok) {
                    heldSearches = [];
                } else {
                    throw Error("Error al guardar");
                }
            } catch (error) {
                console.error("Error:", error);
            }
        }
    }

    function calcularPeso(data: Array<string>) {
        const jsonData = JSON.stringify(data);
        const bytes = new Blob([jsonData]).size;
        return (bytes / 1024).toFixed(2);
    }

    const HandleQuery = (event: CustomEvent<{ query_trimmed: string }>) => {
        let input = document.getElementById(
            "search-input",
        ) as HTMLInputElement | null;
        if (input) {
            input.value = event.detail.query_trimmed;
            input.focus();
        }
    };

    onMount(() => {
        document.addEventListener("select-query", HandleQuery);
        return () => {
            document.removeEventListener("select-query", HandleQuery);
        };
    });
</script>

<main class="main-content">
    <HistoryFloating />
    <div class="legend">
        <div class="legend-title">Atajos</div>
        <div class="legend-item">
            <span class="kbssample"> Ctrl+b</span>
            <span class="legend-text">Buscar</span>
        </div>
        <div class="legend-item">
            <span class="kbssample">Ctrl+h</span>
            <span class="legend-text">Abrir History</span>
        </div>
        <div class="legend-item">
            <span class="kbssample">Ctrl+Shift+s</span>
            <span class="legend-text">Descargar CSV</span>
        </div>

        <div class="legend-item">
            <span class="kbssample">Ctrl+Shift+f</span>
            <span class="legend-text">Añadir a Favorites</span>
        </div>
    </div>

    <div class="form-container">
        <form onsubmit={handleSearch} class="search-form" id="search-form">
            <!-- Top row: Configuration options -->
            <div class="config-options">
                <div class="search-group">
                    <label for="strategy">Método de Búsqueda:</label>
                    <select
                        id="strategy"
                        name="strategy"
                        class="search-type"
                        bind:value={strategy}
                        onchange={handleStrategyChange}
                    >
                        <option value="Fts">Full Text Search</option>
                        <option value="Semantic">Semántica</option>
                        <option value="ReciprocalRankFusion"
                            >Reciprocal Rank Fusion</option
                        >
                    </select>
                </div>

                {#if showOcultables}
                    <div class="search-group ocultable">
                        <label for="vecinos">
                            N° de Vecinos:
                            <span class="help-icon">
                                i
                                <span class="search-tooltip"
                                    >Representa el número de resultados más
                                    cercanos que se quiere buscar.</span
                                >
                            </span>
                        </label>
                        <div class="age-range">
                            <input
                                type="number"
                                id="vecinos"
                                name="k"
                                min="1"
                                max="4096"
                                bind:value={k}
                            />
                        </div>
                    </div>
                {:else}
                    <input type="hidden" name="k" value={k} />
                {/if}

                <input
                    type="hidden"
                    name="document"
                    value={$selectedDocument}
                />

                {#if showBalanceSlider}
                    <div class="search-group balance-slider">
                        <label for="balanceSlider">
                            Pesos:
                            <span class="help-icon">
                                i
                                <span class="search-tooltip"
                                    >Representa el compromiso de asignarle más
                                    importancia a los resultados de cada
                                    búsqueda.</span
                                >
                            </span>
                        </label>
                        <input
                            type="range"
                            id="balanceSlider"
                            min="0"
                            max="100"
                            bind:value={sliderValue}
                            oninput={updateSliderValues}
                        />
                        <p>
                            Peso FTS: <span
                                id="value1Display"
                                class="slider-value">{peso_fts}</span
                            >
                        </p>
                        <p>
                            Peso Semantic: <span
                                id="value2Display"
                                class="slider-value">{peso_semantic}</span
                            >
                        </p>
                    </div>
                {/if}

                <div class="search-group">
                    <input
                        type="hidden"
                        id="hiddenValue1"
                        name="peso_fts"
                        value={peso_fts}
                    />
                    <input
                        type="hidden"
                        id="hiddenValue2"
                        name="peso_semantic"
                        value={peso_semantic}
                    />
                </div>
            </div>

            <!-- Middle row: Search bar (full width) -->
            <div class="search-group search-bar full-width">
                <label for="search-input">
                    Búsqueda:
                    <span class="help-icon">
                        i
                        <span class="search-tooltip">
                            [busqueda], [filtro 1:valor], ..., [filtro n:valor]
                        </span>
                    </span>
                </label>
                <input
                    type="text"
                    class="search-input"
                    id="search-input"
                    placeholder="Ingresa tu busqueda..."
                    name="query"
                    bind:value={query}
                    required
                />
                <p id="validationMessage" class="validation-message">
                    Por favor ingrese un valor
                </p>
            </div>

            {#if error}
                <div class="error-box">
                    <p>{renderSearchError(error)}</p>
                </div>
            {/if}

            <!-- Bottom row: Buttons -->
            <div class="button-container">
                <button
                    aria-label="Buscar"
                    title="Buscar"
                    type="submit"
                    class="btn search-button">Buscar</button
                >
                <button
                    aria-label="Descargar"
                    title="Descargar"
                    id="downloadBtn"
                    class="btn search-button"
                    disabled={downloadBtnDisabled}
                    onclick={descargarCSVGlobal}
                >
                    Descargar <span id="itemsCount">{itemsCount}</span>
                    elementos (<span id="dataWeight">{dataWeight}</span>)
                </button>
                <button
                    aria-label="Resetear descarga"
                    title="Resetear descarga"
                    id="resetDownloadBtn"
                    class="btn btn-icon"
                    disabled={downloadBtnDisabled}
                    onclick={resetHeldData}
                >
                    <svg
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        ><path d="M23 4v6h-6" /><path d="M1 20v-6h6" /><path
                            d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"
                        /></svg
                    >
                </button>
                <button
                    aria-label="Agregar búsqueda a favorites"
                    title="Agregar búsqueda a favorites"
                    id="saveBtn"
                    class="btn btn-icon"
                    disabled={downloadBtnDisabled}
                    onclick={saveFavorite}
                >
                    <svg
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        ><polygon
                            points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"
                        ></polygon></svg
                    >
                </button>
            </div>

            <div class="tooltip">
                Presiona Ctrl + b para empezar a buscar. Presiona Ctrl + Shift +
                s para descargar.
            </div>
        </form>
    </div>

    <div id="table-content">
        <Table table={$tableContent} />
    </div>
</main>
