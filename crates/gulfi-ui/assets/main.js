let heldData = [];
let heldHeaders = [];
let isHolding = false;

document.addEventListener("DOMContentLoaded", async () => {
    await updateHistorial();
    initHistorial();
    initKeyboard();
    updateForm();
    hideElements();
    initSlider();

    document.body.addEventListener('htmx:afterRequest', async () => {
        await updateHistorial();
        initHistorial();
        guardarResultados();
        initPagination();
    });
});

function initHistorial() {
    const historialItems = document.querySelectorAll(".historial-item");
    for (const item of historialItems) {
        item.addEventListener("click", () => {
            const queryContent = item.textContent || "";
            document.getElementById("search-input").value = queryContent.trim();
        });
    }
}

/**
 * Inicializa los atajos de teclado para la página.
 */
function initKeyboard() {
    document.addEventListener(
        "keydown",
        (event) => {
            const input_search = document.getElementById("search-input");
            if (event.ctrlKey && event.key === "b") {
                event.preventDefault();
                input_search.focus();
            }
        },
        false,
    );

    window.addEventListener("keydown", (event) => {
        if (event.ctrlKey && event.shiftKey && (event.key.toLowerCase() === "s")) {
            event.preventDefault();
            event.stopImmediatePropagation();
            descargarCSV();
            return false;
        }
    }, {
        capture: true,
        passive: false
    });
}

/**
 * Inicializa la paginación para una tabla de clase 'modern-table'.
 */
function initPagination() {
    const content = document.querySelector(".modern-table");
    if (!content) {
        return;
    }
    const itemsPerPage = 10;
    let currentPage = 0;
    const items = Array.from(content.getElementsByTagName("tr")).slice(1);
    const totalPages = Math.ceil(items.length / itemsPerPage);
    const pagination_container = document.querySelector(".pagination");
    create_pagination_controls(pagination_container, totalPages, show_page);

    function show_page(page) {
        const startIndex = page * itemsPerPage;
        const endIndex = startIndex + itemsPerPage;

        items.forEach((item, index) => {
            item.style.display =
                index >= startIndex && index < endIndex ? "" : "none";
        });

        update_pagination_info(page, totalPages);
    }

    show_page(currentPage);

    function create_pagination_controls(container, total, show_page_callback) {
        const startButton = document.createElement("button");
        startButton.textContent = "<<";
        startButton.addEventListener("click", () => {
            currentPage = 0;
            show_page_callback(currentPage);
        });

        const prevButton = document.createElement("button");
        prevButton.textContent = "<";
        prevButton.addEventListener("click", () => {
            if (currentPage > 0) {
                show_page_callback(--currentPage);
            }
        });

        const pageInfo = document.createElement("span");
        pageInfo.classList.add("page-info");

        const nextButton = document.createElement("button");
        nextButton.textContent = ">";
        nextButton.addEventListener("click", () => {
            if (currentPage < totalPages - 1) {
                show_page_callback(++currentPage);
            }
        });

        const endButton = document.createElement("button");
        endButton.textContent = ">>";
        endButton.addEventListener("click", () => {
            currentPage = totalPages - 1;
            show_page_callback(currentPage);
        });

        container.append(startButton, prevButton, pageInfo, nextButton, endButton);
        update_pagination_info(currentPage, total);
    }

    function update_pagination_info(page, total) {
        const pageInfo = document.querySelector(".page-info");
        pageInfo.textContent = `Página ${page + 1} de ${total}`;
    }
}

function hideElements() {
    const ocultables = document.querySelectorAll(".ocultable");
    const strategy = document.getElementById("strategy");
    const balance_slider = document.querySelector(".balance-slider");

    // Initial setup based on the current value of strategy
    if (strategy.value === "ReciprocalRankFusion") {
        balance_slider.style.display = "block";
        for (const item of ocultables) {
            item.style.display = "block";
        }
    } else {
        balance_slider.style.display = "none";
        if (strategy.value === "Fts") {
            for (const item of ocultables) {
                item.style.display = "none";
            }
        } else {
            for (const item of ocultables) {
                item.style.display = "block";
            }
        }
    }

    // Attach event listener
    strategy.addEventListener("change", () => {
        if (strategy.value === "ReciprocalRankFusion") {
            balance_slider.style.display = "block";
            for (const item of ocultables) {
                item.style.display = "block";
            }
        } else {
            balance_slider.style.display = "none";
            if (strategy.value === "Fts") {
                for (const item of ocultables) {
                    item.style.display = "none";
                }
            } else {
                for (const item of ocultables) {
                    item.style.display = "block";
                }
            }
        }
    });
}

/**
 * Inicializa el form para la búsqueda, pre-completando valores a partir de la URL si es posible.
 */
function updateForm() {
    const searchConfig = getUrlParams();

    if (Object.keys(searchConfig).length === 0) {
        return;
    }

    document.getElementById("search-input").value = searchConfig.query;
    document.getElementById("age_min").value = searchConfig.edad_min;
    document.getElementById("age_max").value = searchConfig.edad_max;
    document.getElementById("balanceSlider").value = searchConfig.peso_fts || 50;
    document.getElementById("vecinos").value = searchConfig.k;

    const strategy = document.getElementById("strategy");
    strategy.value = searchConfig.strategy;

    document.getElementById("value1Display").textContent = searchConfig.peso_fts;
    document.getElementById("value2Display").textContent =
        searchConfig.peso_semantic;

    document.getElementById("hiddenValue1").value = searchConfig.peso_fts;
    document.getElementById("hiddenValue2").value = searchConfig.peso_semantic;

    const sexoRadios = document.getElementsByName("sexo");
    for (const radio of sexoRadios) {
        if (radio.value === searchConfig.sexo) {
            radio.checked = true;
        }
    }
}

/**
 *  Parsea los parámetros URL y los devuelve como un objeto.
 */
function getUrlParams() {
    const params = new URLSearchParams(window.location.search);
    const searchConfig = {};
    for (const [key, value] of params) {
        searchConfig[key] = value;
    }
    return searchConfig;
}

function initSlider() {
    const balance_slider = document.getElementById("balanceSlider");
    const peso_fts_label = document.getElementById("value1Display");
    const peso_semantic_label = document.getElementById("value2Display");
    const hiddenValue1 = document.getElementById("hiddenValue1");
    const hiddenValue2 = document.getElementById("hiddenValue2");

    if (
        !balance_slider ||
        !peso_fts_label ||
        !peso_semantic_label ||
        !hiddenValue2 ||
        !hiddenValue1
    ) {
        return;
    }

    const updateValues = () => {
        const variable1Value = balance_slider.value;
        const variable2Value = 100 - variable1Value;

        peso_fts_label.textContent = variable1Value;
        peso_semantic_label.textContent = variable2Value;

        hiddenValue1.value = variable1Value;
        hiddenValue2.value = variable2Value;
    };

    balance_slider.addEventListener("input", updateValues);
}

async function updateHistorial() {
    const historial = document.getElementById("historial");

    if (!historial) {
        console.error("Elemento #historial no encontrado en el DOM.");
        return;
    }

    try {
        const response = await fetch("/historial");

        if (!response.ok) {
            historial.innerHTML = "<li>Ha ocurrido un error.</li>"
            return;
        }

        const data = await response.json();
        historial.innerHTML = "";

        if (data.length === 0) {
            historial.innerHTML = "<li>No se encuentran elementos.</li>"
            return;
        }

        for (const el of data) {
            const listItem = document.createElement("li");
            listItem.textContent = el.query;
            listItem.classList.add("historial-item");
            historial.appendChild(listItem);
        }

    }
    catch (error) {
        historial.innerHTML = "<li>Ha ocurrido un error.</li>"
    }
}

function guardarResultados() {
    const table = document.getElementById('table-content');
    const itemsCount = document.getElementById('itemsCount');
    const dataWeight = document.getElementById('dataWeight');
    const downloadBtn = document.getElementById('downloadBtn');

    const headers = table.querySelectorAll('thead th');

    headers.forEach((header, index) => {
        header.addEventListener('mousedown', () => {
            if (!isHolding) {
                isHolding = true;

                heldHeaders.push(header.innerText);

                heldData.push(...[...table.querySelectorAll('tbody tr')].map(row => {
                    const cell = row.children[index];
                    return cell ? cell.innerText : '';
                }));

                const weight = calcularPeso(heldData);
                itemsCount.textContent = heldData.length;
                dataWeight.textContent = `${weight} KB`;

                downloadBtn.disabled = false;
            }
        });

        header.addEventListener('mouseup', () => {
            isHolding = false;
        });
    });

    downloadBtn.addEventListener('click', () => {
        descargarCSV()
    });
}

function descargarCSV() {
    if (heldData.length === 0) {
        return;
    }
    const csvRows = [];
    csvRows.push(heldData.join(','));

    const csvString = csvRows.join('\n');
    const blob = new Blob([csvString], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'busqueda.csv';
    a.click();
    URL.revokeObjectURL(url);

    heldData = [];
    heldHeaders = [];
    itemsCount.textContent = 0;
    dataWeight.textContent = "0 KB";
}

function calcularPeso(data) {
    const jsonData = JSON.stringify(data);
    const bytes = new Blob([jsonData]).size;
    return (bytes / 1024).toFixed(2);
}
