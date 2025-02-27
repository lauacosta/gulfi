<script lang="ts">
    import { RouterView } from "@dvcol/svelte-simple-router/components";
    import type {
        Route,
        RouterOptions,
    } from "@dvcol/svelte-simple-router/models";
    import Home from "./routes/Home.svelte";
    import Fallback from "./routes/Fallback.svelte";
    import Favoritos from "./routes/Favoritos.svelte";

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
</script>

<header class="top-header">
    <a href="/" class="header-icon">
        <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            ><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"
            ></path><polyline points="9 22 9 12 15 12 15 22"></polyline></svg
        >
        Inicio
    </a>
    <a href="/favoritos" class="header-icon">
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
        Favoritos
    </a>
</header>

<main>
    <RouterView {options} />
</main>
