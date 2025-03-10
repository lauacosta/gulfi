<script lang="ts">
    import { RouterView } from "@dvcol/svelte-simple-router/components";
    import type {
        Route,
        RouterOptions,
    } from "@dvcol/svelte-simple-router/models";
    import Home from "./routes/Home.svelte";
    import Fallback from "./routes/Fallback.svelte";
    import Layout from "./lib/Layout.svelte";
    import Sidebar from "./lib/Sidebar.svelte";
    import Favoritos from "./routes/Favoritos.svelte";
    import { onMount } from "svelte";

    const RouteName = {
        Home: "home",
        Favoritos: "favoritos",
        Fallback: "fallback",
    } as const;

    type RouteNames = (typeof RouteName)[keyof typeof RouteName];

    export const routes: Readonly<Route<RouteNames>[]> = [
        {
            name: RouteName.Home,
            path: "/",
            component: Home,
        },

        {
            name: RouteName.Favoritos,
            path: "/favoritos",
            component: Favoritos,
        },

        {
            name: RouteName.Fallback,
            path: "*",
            component: Fallback,
        },
    ] as const;

    export const options: RouterOptions<RouteNames> = { routes } as const;

    // function initKeyboard() {
    //     window.addEventListener(
    //         "keydown",
    //         (event) => {
    //             if (event.ctrlKey && event.key.toLowerCase() === "h") {
    //                 event.preventDefault();
    //                 event.stopImmediatePropagation();
    //                 console.log("Hola");
    //                 return false;
    //             }
    //         },
    //         { capture: true, passive: false },
    //     );
    // }
    //
    // onMount(() => {
    //     initKeyboard();
    // });
</script>

<Layout>
    <Sidebar />
    <RouterView {options} />
</Layout>
