use askama::{Error, Template};
use gulfi_shared::SearchStrategy;
use include_dir::{Dir, include_dir};

pub static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates/assets");

#[derive(Template, Default)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub is_expanded: bool,
    pub active_route: String,
    pub strategy: SearchStrategy,
    pub show_ocultables: bool,
    pub selected_document: String,
    pub search_error: Option<String>,
    pub is_streaming: bool,
    pub is_loading: bool,
    pub peso_fts: u32,
    pub peso_semantic: u32,
    pub k: u32,
}

impl IndexTemplate {
    pub fn has_search_error(&self) -> bool {
        self.search_error.is_some()
    }

    pub fn search_error_message(&self) -> &str {
        self.search_error.as_deref().unwrap_or("")
    }

    pub fn renderr(self) -> Result<String, Error> {
        self.render()
    }
}

#[cfg(test)]
mod tests {
    use expect_test::{Expect, expect};

    use super::*;

    fn check<T: Template>(template: T, expect: Expect) {
        let res = template.render().unwrap();
        expect.assert_debug_eq(&res);
    }

    #[test]
    fn test_render() {
        check(
            IndexTemplate::default(),
            expect![[r#"
                "<!doctype html>\n<html lang=\"en\">\n    <head>\n        <meta charset=\"utf-8\" />\n        <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n        <link rel=\"icon\" href=\"./public/favicon.ico\" type=\"image/x-icon\" />\n        <link rel=\"preconnect\" href=\"https://fonts.googleapis.com\" />\n        <link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin /> <link\n            href=\"https://fonts.googleapis.com/css2?family=Roboto+Mono:ital,wght@0,100..700;1,100..700&display=swap\"\n            rel=\"stylesheet\"\n        />\n        <link rel=\"stylesheet\" href=\"/assets/styles.css\">\n        <title>Gulfi</title>\n    </head>\n  <body>\n    <div class=\"top-bar\">\n        <label>\n            Documento:\n        </label>\n    </div>\n\n    <div class=\"content-wrapper\">\n        <!-- Sidebar -->\n        <aside class=\"sidebar collapsed\">\n            <button class=\"toggle-btn\" onclick=\"this.classList.toggle('expanded')\">\n                <!-- TODO: Move to icons -->\n                <svg class=\"icon-collapsed\"\n                    xmlns=\"http://www.w3.org/2000/svg\"\n                    width=\"24\" height=\"24\" viewBox=\"0 0 24 24\"\n                    fill=\"none\" stroke=\"currentColor\"\n                    stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">\n                    <rect width=\"18\" height=\"18\" x=\"3\" y=\"3\" rx=\"2\" />\n                    <path d=\"M9 3v18\" />\n                    <path d=\"m16 15-3-3 3-3\" />\n                </svg>\n                <svg class=\"icon-expanded\"\n                    xmlns=\"http://www.w3.org/2000/svg\"\n                    width=\"24\" height=\"24\" viewBox=\"0 0 24 24\"\n                    fill=\"none\" stroke=\"currentColor\"\n                    stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">\n                    <rect width=\"18\" height=\"18\" x=\"3\" y=\"3\" rx=\"2\" />\n                    <path d=\"M9 3v18\" />\n                    <path d=\"m14 9 3 3-3 3\" />\n                </svg>\n            </button>\n\n                <nav>\n                  <ul>\n                    <li class=\"\">\n                        <a href=\"/\">\n                            <span class=\"icon\"><svg\n    xmlns=\"http://www.w3.org/2000/svg\"\n    width=\"24\"\n    height=\"24\"\n    viewBox=\"0 0 24 24\"\n    fill=\"none\"\n    stroke=\"currentColor\"\n    stroke-width=\"2\"\n    stroke-linecap=\"round\"\n    stroke-linejoin=\"round\"\n>\n    <circle cx=\"11\" cy=\"11\" r=\"8\" />\n    <path d=\"m21 21-4.3-4.3\" />\n</svg>\n</span>\n                            \n                        </a>\n                    </li>\n\n                    <li class=\"\">\n                        <a href=\"/favorites\">\n                            <span class=\"icon\"><svg\n    xmlns=\"http://www.w3.org/2000/svg\"\n    width=\"24\"\n    height=\"24\"\n    viewBox=\"0 0 24 24\"\n    fill=\"none\"\n    stroke=\"currentColor\"\n    stroke-width=\"2\"\n    stroke-linecap=\"round\"\n    stroke-linejoin=\"round\"\n>\n    <path\n        d=\"M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z\"\n    />\n</svg>\n</span>\n                            \n                        </a>\n                    </li>\n\n                    <li class=\"\">\n                        <a href=\"/history\">\n                            <span class=\"icon\"><svg\n    xmlns=\"http://www.w3.org/2000/svg\"\n    width=\"24\"\n    height=\"24\"\n    viewBox=\"0 0 24 24\"\n    fill=\"none\"\n    stroke=\"currentColor\"\n    stroke-width=\"2\"\n    stroke-linecap=\"round\"\n    stroke-linejoin=\"round\"\n>\n    <path d=\"M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8\" />\n    <path d=\"M3 3v5h5\" />\n    <path d=\"M12 7v5l4 2\" />\n</svg>\n</span>\n                            \n                        </a>\n                    </li>\n                </ul>\n            </nav>\n\n            </aside>\n\n            <main class=\"main-content\">\n            <!-- TODO: ASKAMA TO THE RESCUE -->\n            <!-- <HistoryFloating /> -->\n            \n\n            <div class=\"legend\">\n                <div class=\"legend-title\">Atajos</div>\n                    <div class=\"legend-item\">\n                        <span class=\"kbssample\">Ctrl+b</span>\n                        <span class=\"legend-text\">Search</span>\n                    </div>\n                    <div class=\"legend-item\">\n                        <span class=\"kbssample\">Ctrl+h</span>\n                        <span class=\"legend-text\">Open history</span>\n                    </div>\n                    <div class=\"legend-item\">\n                        <span class=\"kbssample\">\"Ctrl+Shift+s</span>\n                        <span class=\"legend-text\">Download as CSV</span>\n                    </div>\n                    <div class=\"legend-item\">\n                        <span class=\"kbssample\">Ctrl+Shift+f</span>\n                        <span class=\"legend-text\">Add to Favorites</span>\n                    </div>\n                </div>\n            </div>\n\n            <div class=\"form-container\">\n                <form method=\"POST\" action=\"/search\">\n                    <div class=\"config-options\">\n                        <div class=\"search-group\">\n                            <label for=\"strategy\">Método de Búsqueda:</label>\n\n                            <select id=\"strategy\" name=\"strategy\">\n                                <option value=\"Fts\"\n                                    selected\n                                >\n                                    Full Text Search\n                                </option>\n\n                                <option value=\"Semantic\"\n                                    \n                                >\n                                    Semantic\n                                </option>\n\n                                <option value=\"ReciprocalRankFusion\"\n                                    \n                                >\n                                    Hybrid\n                            </select>\n                        </div>\n\n\n                        \n                            <input type=\"hidden\" name=\"k\" value=\"0\"/>\n                        \n\n                        \n\n                        <input type=\"hidden\" name=\"document\" value=\"\"/>\n                    </div>\n\n                    <div class=\"search-group search-bar full-width\">\n                        <label for=\"search-input\">Búsqueda:</label>\n                        <input\n                            type=\"text\"\n                            id=\"search-input\"\n                            name=\"query\"\n                            placeholder=\"Ingresa tu busqueda...\"\n                            required\n                        />\n                    </div>\n\n                    \n\n\n                    <div class=\"button-container\">\n                        \n                            <button type=\"submit\" class=\"btn search-button\">\n                                Buscar\n                            </button>\n                        \n                    </div>\n\n                </form>\n            </div>\n\n            <div>\n            <!-- TODO: ASKAMA TO THE RESCUE -->\n                <!-- <Table --> \n                <!-- table={$tableContent} --> \n                <!-- isLoading={searchState.isLoading} -->\n                <!-- isStreaming={searchState.isStreaming} -->\n                <!-- streamingCount={streamingResults.length} -->\n                <!-- /> -->\n                \n            </div>\n            </main>\n    </div>\n    <!-- <script type=\"module\" src=\"/src/main.js\"></script> -->\n  </body>\n</html>\n"
            "#]],
        );
    }
}
