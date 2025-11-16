// import { svelte } from "@sveltejs/vite-plugin-svelte";

// import { defineConfig } from "vite";

// // https://vite.dev/config/
// export default defineConfig({
// 	plugins: [svelte()],
// });

import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import fs from "node:fs";
import path from "node:path";

function getComponentEntries() {
    const dir = path.resolve("src/lib");
    const files = fs.readdirSync(dir).filter((f) => f.endsWith(".svelte"));

    const entries = {};
    for (const file of files) {
        const name = file.replace(".svelte", "");
        entries[name] = path.join(dir, file);
    }
    return entries;
}

export default defineConfig({
    plugins: [svelte()],
    build: {
        lib: false, // not a single library
        rollupOptions: {
            input: getComponentEntries(),
            output: {
                entryFileNames: "components/[name].js",
                format: "esm",
            },
        },
    },
});
