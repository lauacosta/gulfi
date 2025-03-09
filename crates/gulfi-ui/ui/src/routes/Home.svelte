<script lang="ts">
    import { onMount } from "svelte";
    import Table from "../lib/Table.svelte";
    import type { TableContent } from "../lib/types";
    import type { SearchResponse } from "../lib/types";
    import { writable } from "svelte/store";

    const apiUrl = import.meta.env.VITE_API_URL;

    let search_form;
    let base64_embedding: string | null = $state(null);
    let total_pages = $state(0);
    let page = $state(1);
    const tableContent = writable<TableContent>({
        msg: "",
        columns: [],
        rows: [],
    });

    let heldData: Array<string> = [];
    let heldHeaders = [];
    let heldSearches = [];
    let isHolding = false;
    let historialItems = [];
    let itemsCount = $state(0);
    let dataWeight = $state("0 KB");
    let downloadBtnDisabled = $state(true);

    let query = $state("");
    let strategy = $state("Fts");
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
        // await updateHistorial();
        hideElements();
        initKeyboard();
    });

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

    // async function updateHistorial() {
    //     try {
    //         const response = await fetch(`${apiUrl}/api/historial`);
    //
    //         if (!response.ok) {
    //             historialItems = [
    //                 { query: "Ha ocurrido un error.", isError: true },
    //             ];
    //             return;
    //         }
    //
    //         const data = await response.json();
    //
    //         if (data.length === 0) {
    //             historialItems = [
    //                 { query: "No se encuentran elementos.", isError: true },
    //             ];
    //             return;
    //         }
    //
    //         historialItems = data.map((item) => ({ ...item, isError: false }));
    //     } catch (error) {
    //         historialItems = [
    //             { query: "Ha ocurrido un error.", isError: true },
    //         ];
    //     }
    // }

    function selectHistorialItem(queryText: string) {
        query = queryText.trim();
    }

    async function deleteHistorialItem(queryText: string, index: number) {
        try {
            const deleteResponse = await fetch(
                `${apiUrl}/api/historial?query=${encodeURIComponent(queryText)}`,
                {
                    method: "DELETE",
                },
            );

            if (deleteResponse.ok) {
                historialItems = historialItems.filter((_, i) => i !== index);
            } else {
                throw Error("Error al eliminar el elemento.");
            }
        } catch (error) {
            console.error(
                "Ha ocurrido un error al intentar eliminar el elemento.",
            );
        }
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
    }

    async function handleSearch(event: SubmitEvent, page: number) {
        event?.preventDefault();

        const formData = new FormData(search_form);
        const searchParams = new URLSearchParams();

        for (const [key, value] of formData.entries()) {
            searchParams.append(key, value.toString());
            console.log(key);
            console.log(value.toString());
        }

        searchParams.append("page", page.toString());
        console.log(page.toString());

        if (base64_embedding) {
            console.log(`Base64: ${base64_embedding}`);
            searchParams.append("vector", base64_embedding);
        }

        try {
            const response = await fetch(
                `${apiUrl}/api/search?${searchParams.toString()}`,
            );

            console.log(searchParams);
            if (response.ok) {
                const data: SearchResponse = await response.json();

                tableContent.set(data.table);
                total_pages = data.pages;
                base64_embedding = data.embedding;
                console.log(`After GET REQUEST: ${base64_embedding}`);

                requestAnimationFrame(() => {
                    insertPaginationButtons();
                    guardarResultados();
                    // updateHistorial();
                });
            }
        } catch (error) {
            console.error("Error en la búsqueda:", error);
        }
    }

    function insertPaginationButtons() {
        const content = document.querySelector(".modern-table");
        if (!content) return;

        const paginationContainer = document.querySelector(".pagination");
        if (!paginationContainer) return;

        paginationContainer.innerHTML = "";

        const startButton = document.createElement("button");
        startButton.textContent = "<<";
        startButton.addEventListener("click", (e) => {
            e.preventDefault();
            page = 1;
            handleSearch(null, page);
        });

        const prevButton = document.createElement("button");
        prevButton.textContent = "<";
        prevButton.addEventListener("click", (e) => {
            e.preventDefault();
            if (page === 1) {
                return;
            }
            page -= 1;
            handleSearch(null, page);
        });

        const nextButton = document.createElement("button");
        nextButton.textContent = ">";
        nextButton.addEventListener("click", (e) => {
            e.preventDefault();
            if (page == total_pages) {
                return;
            }
            page += 1;
            handleSearch(null, page);
        });

        const endButton = document.createElement("button");
        endButton.textContent = ">>";
        endButton.addEventListener("click", (e) => {
            e.preventDefault();

            page = total_pages;
            handleSearch(null, page);
        });

        paginationContainer.append(
            startButton,
            prevButton,
            nextButton,
            endButton,
        );
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
                        heldSearches.push(query);
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

        const csvRows = [];
        csvRows.push(heldData.join(","));

        const csvString = csvRows.join("\n");
        const blob = new Blob([csvString], { type: "text/csv" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "busqueda.csv";
        a.click();
        URL.revokeObjectURL(url);

        resetHeldData();
        heldSearches = [];
    }

    async function saveFavorite() {
        if (heldData.length === 0 || heldSearches.length === 0) return;

        const input = prompt("Ingresa un nombre para guardarlo");
        const name = input?.replace(/[^a-zA-Z_\-\s]/g, "") || "ERROR";

        if (name !== null && name !== "") {
            const data = {
                nombre: name,
                data: JSON.stringify(heldData),
                busquedas: JSON.stringify(heldSearches),
            };

            try {
                const response = await fetch(`${apiUrl}/api/favoritos`, {
                    method: "POST",
                    headers: {
                        "Content-Type": "application/json",
                    },
                    body: JSON.stringify(data),
                });

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
</script>

<main class="main-content">
    <div class="form-container">
        <form
            bind:this={search_form}
            onsubmit={(event) => handleSearch(event, page)}
            class="search-form"
            id="search-form"
        >
            <div class="search-group">
                <label for="search-input"
                    >Búsqueda:
                    <span class="help-icon">
                        i
                        <span class="search-tooltip"
                            >El formato es "query | provincia, ciudad", los
                            campos provincia y ciudad son opcionales.</span
                        >
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
            </div>

            <div class="search-group">
                <label for="strategy">Método de Búsqueda:</label>
                <select
                    id="strategy"
                    name="strategy"
                    class="search-type"
                    bind:value={strategy}
                    onchange={hideElements}
                >
                    <option value="Fts">Full Text Search</option>
                    <option value="Semantic">Semántica</option>
                    <option value="ReciprocalRankFusion"
                        >Reciprocal Rank Fusion</option
                    >
                    <option value="KeywordFirst">Keyword First</option>
                    <option value="ReRankBySemantics"
                        >Re-Ranking by Semantics</option
                    >
                </select>
            </div>

            <div class="search-group">
                <label>Sexo:</label>
                <div class="radio-group">
                    <label>
                        <input
                            type="radio"
                            name="sexo"
                            value="U"
                            bind:group={sexo}
                        /> Todos
                    </label>
                    <label>
                        <input
                            type="radio"
                            name="sexo"
                            value="M"
                            bind:group={sexo}
                        /> Masculino
                    </label>
                    <label>
                        <input
                            type="radio"
                            name="sexo"
                            value="F"
                            bind:group={sexo}
                        /> Femenino
                    </label>
                </div>
            </div>

            <div class="search-group">
                <label>Rango de Edad:</label>
                <div class="age-range">
                    <input
                        type="number"
                        id="age_min"
                        name="edad_min"
                        min="0"
                        max="100"
                        bind:value={edad_min}
                        placeholder="Mínimo"
                    />
                    <input
                        type="number"
                        id="age_max"
                        name="edad_max"
                        min="0"
                        max="100"
                        bind:value={edad_max}
                        placeholder="Máximo"
                    />
                </div>
            </div>

            {#if showOcultables}
                <div class="search-group ocultable">
                    <label for="vecinos"
                        >N° de Vecinos:
                        <span class="help-icon">
                            i
                            <span class="search-tooltip"
                                >Representa el número de resultados más cercanos
                                que se quiere buscar.</span
                            >
                        </span>
                    </label>
                    <div class="age-range">
                        <input
                            type="number"
                            id="vecinos"
                            name="k"
                            min="1"
                            max="10000"
                            bind:value={k}
                        />
                    </div>
                </div>
            {/if}

            {#if showBalanceSlider}
                <div class="search-group balance-slider">
                    <label for="balanceSlider"
                        >Pesos:
                        <span class="help-icon">
                            i
                            <span class="search-tooltip"
                                >Representa el compromiso de asignarle más
                                importancia a los resultados de cada búsqueda.</span
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
                        Peso FTS: <span id="value1Display" class="slider-value"
                            >{peso_fts}</span
                        >
                    </p>
                    <p>
                        Peso Semantic: <span
                            id="value2Display"
                            class="slider-value">{peso_semantic}</span
                        >
                    </p>
                </div>
            {:else}
                <input type="hidden" name="k" value={k} />
            {/if}

            <input type="hidden" name="limit" value="10" />

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
                    aria-label="Agregar búsqueda a favoritos"
                    title="Agregar búsqueda a favoritos"
                    id="saveBtn"
                    class="btn btn-icon"
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
        </form>
        <div class="tooltip">
            Presiona Ctrl + b para empezar a buscar. Presiona Ctrl + Shift + s
            para descargar.
        </div>
    </div>

    <div id="table-content">
        <Table table={$tableContent} {total_pages} {page} />
    </div>
</main>
