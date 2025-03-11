
export function getBgColor(strategy: string) {
    switch (strategy) {
        case "Fts":
            return "fts";
        case "ReciprocalRankFusion":
            return "reciprocal-rank-fusion";
        case "Semantic":
            return "semantic";
        default:
            return "";
    }
}

export function inputPopUp(msg: string): Promise<string | null> {
    return new Promise((resolve) => {
        const modal = document.createElement("div");
        modal.style.position = "fixed";
        modal.style.top = "0";
        modal.style.left = "0";
        modal.style.width = "100vw";
        modal.style.height = "100vh";
        modal.style.background = "rgba(0,0,0,0.6)";
        modal.style.display = "flex";
        modal.style.justifyContent = "center";
        modal.style.alignItems = "center";
        modal.style.zIndex = "1000";
        modal.style.opacity = "0";
        modal.style.transition = "opacity 0.3s ease";

        const modalBox = document.createElement("div");
        modalBox.style.background = "#fff";
        modalBox.style.padding = "24px";
        modalBox.style.borderRadius = "12px";
        modalBox.style.boxShadow = "0 10px 25px rgba(0,0,0,0.2)";
        modalBox.style.textAlign = "center";
        modalBox.style.width = "350px";
        modalBox.style.maxWidth = "90%";
        modalBox.style.transform = "translateY(20px)";
        modalBox.style.transition = "transform 0.3s ease";

        modalBox.innerHTML = `
            <p style="margin: 0 0 16px; font-size: 16px; color: #333; font-weight: 500;">${msg}</p>
            <input type="text" id="popupInput" style="width: 100%; padding: 12px; margin: 10px 0 5px 0; border: 1px solid #ddd; border-radius: 6px; font-size: 14px; box-sizing: border-box; outline: none; transition: all 0.2s;" />
            <p id="validationMessage" style="color: #e74c3c; font-size: 12px; margin: 0 0 10px 0; text-align: left; visibility: hidden; height: 16px;">Por favor ingrese un valor</p>
            <div style="display: flex; justify-content: space-between; margin-top: 5px;">
                <button id="cancelBtn" class="btn delete-button">Cancelar</button>
                <button id="confirmBtn" class="btn search-button">Confirmar</button>
            </div>
        `;

        modal.appendChild(modalBox);
        document.body.appendChild(modal);

        setTimeout(() => {
            modal.style.opacity = "1";
            modalBox.style.transform = "translateY(0)";
        }, 10);

        const inputElement = document.getElementById("popupInput") as HTMLInputElement;
        const validationMessage = document.getElementById("validationMessage");
        inputElement?.focus();

        const confirmBtn = document.getElementById("confirmBtn");
        const cancelBtn = document.getElementById("cancelBtn");

        // Input styling on focus/blur
        if (inputElement) {
            inputElement.onfocus = () => {
                inputElement.style.borderColor = "#4361ee";
                inputElement.style.boxShadow = "0 0 0 2px rgba(67, 97, 238, 0.2)";
            };
            inputElement.onblur = () => {
                if (inputElement.value.trim() === '') {
                    inputElement.style.borderColor = "#ddd";
                    inputElement.style.boxShadow = "none";
                }
            };
            // Hide validation message when user starts typing
            inputElement.oninput = () => {
                if (validationMessage) {
                    validationMessage.style.visibility = "hidden";
                }
                if (inputElement.value.trim() !== '') {
                    inputElement.style.borderColor = "#4361ee";
                }
            };
        }

        // Function to validate input
        const validateInput = (): boolean => {
            if (!inputElement || inputElement.value.trim() === '') {
                // Show validation message
                if (validationMessage) {
                    validationMessage.style.visibility = "visible";
                }
                // Highlight input in red
                inputElement.style.borderColor = "#e74c3c";
                inputElement.style.boxShadow = "0 0 0 2px rgba(231, 76, 60, 0.2)";
                inputElement.focus();

                // Shake effect
                modalBox.style.animation = "shake 0.5s";
                modalBox.style.animationIterationCount = "1";


                // Remove shake animation after it completes
                setTimeout(() => {
                    modalBox.style.animation = "";
                }, 500);

                return false;
            }
            return true;
        };

        const handleKeydown = (event: KeyboardEvent) => {
            if (event.key === "Enter") {
                if (validateInput()) {
                    document.getElementById("confirmBtn")?.click();
                }
            } else if (event.key === "Escape") {
                document.getElementById("cancelBtn")?.click();
            }
        };

        document.addEventListener("keydown", handleKeydown);

        const cleanup = () => {
            document.removeEventListener("keydown", handleKeydown);
        };

        const closeWithAnimation = (callback: () => void) => {
            modal.style.opacity = "0";
            modalBox.style.transform = "translateY(20px)";
            setTimeout(() => {
                callback();
            }, 300);
        };

        if (confirmBtn) {
            confirmBtn.onclick = () => {
                if (validateInput()) {
                    const value = inputElement.value;
                    closeWithAnimation(() => {
                        resolve(value);
                        modal.remove();
                        cleanup();
                    });
                }
            };
        }

        if (cancelBtn) {
            cancelBtn.onclick = () => {
                closeWithAnimation(() => {
                    resolve(null);
                    modal.remove();
                    cleanup();
                });
            };
        }
    });
}
