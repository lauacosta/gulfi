<script lang="ts">
    import { onMount } from "svelte";
    import Table from "../lib/Table.svelte";

    let heldData: Array<string> = [];
    let heldHeaders = [];
    let heldSearches = [];
    let isHolding = false;
    let historialItems = [];
    let itemsCount = 0;
    let dataWeight = "0 KB";
    let downloadBtnDisabled = true;
    let tableContent: {
        msg: string;
        columns: string[];
        rows: string[][];
    } = {
        msg: "",
        columns: [],
        rows: [],
    };

    let query = "";
    let strategy = "Fts";
    let sexo = "U";
    let edad_min = 0;
    let edad_max = 100;
    let k = 1000;
    let sliderValue = 50;
    let peso_fts = 50;
    let peso_semantic = 50;

    let showOcultables = false;
    let showBalanceSlider = false;

    onMount(async () => {
        await updateHistorial();
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

    function handleStrategyChange() {
        hideElements();
    }

    async function updateHistorial() {
        try {
            const response = await fetch("/api/historial");

            if (!response.ok) {
                historialItems = [
                    { query: "Ha ocurrido un error.", isError: true },
                ];
                return;
            }

            const data = await response.json();

            if (data.length === 0) {
                historialItems = [
                    { query: "No se encuentran elementos.", isError: true },
                ];
                return;
            }

            historialItems = data.map((item) => ({ ...item, isError: false }));
        } catch (error) {
            historialItems = [
                { query: "Ha ocurrido un error.", isError: true },
            ];
        }
    }

    function selectHistorialItem(queryText: string) {
        query = queryText.trim();
    }

    async function deleteHistorialItem(queryText: string, index: number) {
        try {
            const deleteResponse = await fetch(
                `/api/historial?query=${encodeURIComponent(queryText)}`,
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

    async function handleSearch(event: SubmitEvent) {
        event.preventDefault();

        const formData = new FormData(event.target as HTMLFormElement);
        const searchParams = new URLSearchParams();

        for (const [key, value] of formData.entries()) {
            searchParams.append(key, value.toString());
        }

        try {
            const response = await fetch(
                `/api/search?${searchParams.toString()}`,
            );
            if (response.ok) {
                const data = await response.json();

                tableContent = {
                    msg: data.msg,
                    columns: data.columns,
                    rows: data.rows,
                };

                requestAnimationFrame(() => {
                    initPagination();
                    guardarResultados();
                    updateHistorial();
                });
            }
        } catch (error) {
            console.error("Error en la búsqueda:", error);
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
                const response = await fetch("/api/favoritos", {
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

<div class="content-wrapper">
    <aside class="sidebar">
        <h2>Historial</h2>
        <ul class="historial" id="historial">
            {#each historialItems as item, index}
                {#if item.isError}
                    <li>{item.query}</li>
                {:else}
                    <li class="list-item">
                        <span on:click={() => selectHistorialItem(item.query)}
                            >{item.query}</span
                        >
                        <button
                            aria-label="Borrar item del historial"
                            type="button"
                            class="delete-btn"
                            on:click={() =>
                                deleteHistorialItem(item.query, index)}
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                viewBox="0 0 448 512"
                                width="24"
                                height="24"
                            >
                                <path
                                    d="M170.5 51.6L151.5 80l145 0-19-28.4c-1.5-2.2-4-3.6-6.7-3.6l-93.7 0c-2.7 0-5.2 1.3-6.7 3.6zm147-26.6L354.2 80 368 80l48 0 8 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-8 0 0 304c0 44.2-35.8 80-80 80l-224 0c-44.2 0-80-35.8-80-80l0-304-8 0c-13.3 0-24-10.7-24-24S10.7 80 24 80l8 0 48 0 13.8 0 36.7-55.1C140.9 9.4 158.4 0 177.1 0l93.7 0c18.7 0 36.2 9.4 46.6 24.9zM80 128l0 304c0 17.7 14.3 32 32 32l224 0c17.7 0 32-14.3 32-32l0-304L80 128zm80 64l0 208c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-208c0-8.8 7.2-16 16-16s16 7.2 16 16zm80 0l0 208c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-208c0-8.8 7.2-16 16-16s16 7.2 16 16zm80 0l0 208c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-208c0-8.8 7.2-16 16-16s16 7.2 16 16z"
                                />
                            </svg>
                        </button>
                    </li>
                {/if}
            {/each}
        </ul>
    </aside>

    <main class="main-content">
        <div class="form-container">
            <form on:submit={handleSearch} class="search-form" id="search-form">
                <div class="search-group">
                    <label for="search-input"
                        >Búsqueda:
                        <span class="help-icon">
                            &#9432;
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
                        on:change={handleStrategyChange}
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
                                &#9432;
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
                                &#9432;
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
                            on:input={updateSliderValues}
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
                {:else}
                    <input type="hidden" name="k" value={k} />
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

                <div class="button-container">
                    <button
                        aria-label="Buscar"
                        title="Buscar"
                        type="submit"
                        class="search-button">Buscar</button
                    >
                    <button
                        aria-label="Descargar"
                        title="Descargar"
                        id="downloadBtn"
                        class="search-button"
                        disabled={downloadBtnDisabled}
                        on:click|preventDefault={descargarCSVGlobal}
                    >
                        Descargar <span id="itemsCount">{itemsCount}</span>
                        elementos (<span id="dataWeight">{dataWeight}</span>)
                    </button>
                    <button
                        aria-label="Resetear descarga"
                        title="Resetear descarga"
                        id="resetDownloadBtn"
                        class="search-button"
                        on:click|preventDefault={resetHeldData}
                    >
                        ⟳
                    </button>
                    <button
                        aria-label="Agregar búsqueda a favoritos"
                        title="Agregar búsqueda a favoritos"
                        id="saveBtn"
                        class="search-button"
                        on:click|preventDefault={saveFavorite}
                    >
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 576 512"
                            width="24"
                            height="24"
                        >
                            <path
                                d="M259.3 17.8L194 150.2 47.9 171.5c-26.2 3.8-36.7 36-17.7 54.6l105.7 103-25 146c-4.5 26.2 23 46 46.4 33.7L288 439.6l131.7 69.2c23.4 12.3 50.9-7.4 46.4-33.7l-25-146 105.7-103c19-18.6 8.5-50.8-17.7-54.6L382 150.2 316.7 17.8c-11.7-23.6-45.7-23.9-57.4 0z"
                            />
                        </svg>
                    </button>
                </div>
            </form>
            <div class="tooltip">
                Presiona Ctrl + b para empezar a buscar. Presiona Ctrl + Shift +
                s para descargar.
            </div>
        </div>

        <div class="table-container">
            <div class="tooltip">
                Haz click en las columnas que quieras exportar a CSV, se
                acumularan hasta que presiones descargar!
            </div>
            <div id="table-content">
                <Table table={tableContent} />
            </div>
            <div class="pagination"></div>
        </div>
    </main>
</div>
