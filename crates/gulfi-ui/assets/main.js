let heldData = [];
let heldHeaders = [];
let heldSearches = [];
let isHolding = false;

document.addEventListener("DOMContentLoaded", async () => {

    await updateHistorial();
    initHistorial();
    initKeyboard();
    hideElements();
    initSlider();
    setSaveButton();

    document.body.addEventListener("htmx:afterRequest", async () => {
        await updateHistorial();
        initHistorial();
        guardarResultados();
        initPagination();
    });

});

function setSaveButton() {
    const saveBtn = document.getElementById("saveBtn");
    if (!saveBtn) {
        return;
    }

    saveBtn.addEventListener("click", async () => {
        event.preventDefault();
        if (heldData.length === 0 || heldSearches.length === 0) {
            return;
        }

        const input = prompt("Ingresa un nombre para guardarlo");
        const name = input?.replace(/[^a-zA-Z_\-\s]/g, "") || "ERROR";

        if (name !== null && name !== "") {
            const data = {
                nombre: name,
                data: JSON.stringify(heldData),
                busquedas: JSON.stringify(heldSearches),
            };
            try {
                const response = await fetch("/favoritos", {
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
    });
}

function initHistorial() {
    const historialItems = document.querySelectorAll(".list-item");
    const input = document.getElementById("search-input")


    for (const item of historialItems) {
        item.addEventListener("click", () => {
            const queryContent = item.textContent || "";
            input.value = queryContent.trim();
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

    window.addEventListener(
        "keydown",
        (event) => {
            if (event.ctrlKey && event.shiftKey && event.key.toLowerCase() === "s") {
                event.preventDefault();
                event.stopImmediatePropagation();
                descargarCSVGlobal();
                return false;
            }
        },
        {
            capture: true,
            passive: false,
        },
    );
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

    if (!ocultables || !strategy || !balance_slider) {
        return;
    }

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
        console.error("no esta #historial");
        return;
    }

    try {
        const response = await fetch("/historial");

        if (!response.ok) {
            historial.innerHTML = "<li>Ha ocurrido un error.</li>";
            return;
        }

        const data = await response.json();
        historial.innerHTML = "";

        if (data.length === 0) {
            historial.innerHTML = "<li>No se encuentran elementos.</li>";
            return;
        }

        for (const el of data) {
            const listItem = document.createElement("li");
            listItem.textContent = el.query;
            listItem.classList.add("list-item");

            const deleteBtn = document.createElement("button");
            deleteBtn.innerHTML = `
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512" width="24" height="24">
                    <path d="M170.5 51.6L151.5 80l145 0-19-28.4c-1.5-2.2-4-3.6-6.7-3.6l-93.7 0c-2.7 0-5.2 1.3-6.7 3.6zm147-26.6L354.2 80 368 80l48 0 8 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-8 0 0 304c0 44.2-35.8 80-80 80l-224 0c-44.2 0-80-35.8-80-80l0-304-8 0c-13.3 0-24-10.7-24-24S10.7 80 24 80l8 0 48 0 13.8 0 36.7-55.1C140.9 9.4 158.4 0 177.1 0l93.7 0c18.7 0 36.2 9.4 46.6 24.9zM80 128l0 304c0 17.7 14.3 32 32 32l224 0c17.7 0 32-14.3 32-32l0-304L80 128zm80 64l0 208c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-208c0-8.8 7.2-16 16-16s16 7.2 16 16zm80 0l0 208c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-208c0-8.8 7.2-16 16-16s16 7.2 16 16zm80 0l0 208c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-208c0-8.8 7.2-16 16-16s16 7.2 16 16z"/>
                </svg>
            `;

            deleteBtn.classList.add("delete-btn");

            // deleteBtn.classList.add("delete-btn");
            deleteBtn.addEventListener("click", async () => {
                try {
                    const deleteResponse = await fetch(`/historial?query=${el.query}`, {
                        method: "DELETE",
                    });

                    if (deleteResponse.ok) {
                        listItem.remove();
                    } else {
                        throw Error("Error al eliminar el elemento.");
                    }
                } catch (error) {
                    console.error(
                        "Ha ocurrido un error al intentar eliminar el elemento.",
                    );
                }
            });

            listItem.appendChild(deleteBtn);
            historial.appendChild(listItem);
        }
    } catch (error) {
        historial.innerHTML = "<li>Ha ocurrido un error.</li>";
    }
}

function guardarResultados() {
    const table = document.getElementById("table-content");
    const itemsCount = document.getElementById("itemsCount");
    const dataWeight = document.getElementById("dataWeight");
    const downloadBtn = document.getElementById("downloadBtn");

    const headers = table.querySelectorAll("thead th");

    headers.forEach((header, index) => {
        header.addEventListener("mousedown", () => {
            if (!isHolding) {
                isHolding = true;

                heldHeaders.push(header.innerText);

                heldData.push(
                    ...[...table.querySelectorAll("tbody tr")].map((row) => {
                        const cell = row.children[index];
                        return cell ? cell.innerText : "";
                    }),
                );

                const weight = calcularPeso(heldData);
                itemsCount.textContent = heldData.length;
                dataWeight.textContent = `${weight} KB`;
                downloadBtn.disabled = false;

                const input = document.getElementById("search-input")
                if (input !== null) {
                    if (input.value.trim()) {
                        heldSearches.push(input.value);
                    }
                }
            }
        });

        header.addEventListener("mouseup", () => {
            isHolding = false;
        });
    });

    if (downloadBtn) {
        downloadBtn.addEventListener("click", async () => {
            descargarCSVGlobal();
        });
    }
}

function descargarCSVGlobal() {
    if (heldData.length === 0) {
        return;
    }
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

    heldData = [];
    heldHeaders = [];
    heldSearches = [];
    itemsCount.textContent = 0;
    dataWeight.textContent = "0 KB";
}

function descargarCSVFavoritos(data, filename) {
    if (!data) {
        return;
    }

    const dataArray = JSON.parse(data);
    if (!dataArray || dataArray.length === 0) {
        return;
    }

    const csvRows = [];

    for (const item of dataArray) {
        csvRows.push(`"${item.replace(/"/g, '""')}"`);
    }

    const csvString = csvRows.join("\n");

    const blob = new Blob([csvString], { type: "text/csv" });

    const link = document.createElement("a");
    link.href = URL.createObjectURL(blob);
    link.download = `${filename}.csv`;
    link.click();
    URL.revokeObjectURL(link.href);
}

function calcularPeso(data) {
    const jsonData = JSON.stringify(data);
    const bytes = new Blob([jsonData]).size;
    return (bytes / 1024).toFixed(2);
}
