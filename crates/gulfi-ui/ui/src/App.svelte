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
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 576 512"
            width="24"
            height="24"
        >
            <path
                d="M280.4 148.3L96 300.1V456c0 13.3 10.7 24 24 24l112-.3c13.3 0 24-10.7 24-24V344c0-13.3 10.7-24 24-24h16c13.3 0 24 10.7 24 24v111.7c0 13.3 10.7 24 24 24l112 .3c13.3 0 24-10.7 24-24V300.1l-184.4-151.8c-10-8.2-25.2-8.2-35.2 0zM571.6 251.5L488 184.4V72c0-13.3-10.7-24-24-24h-56c-13.3 0-24 10.7-24 24v72l-88.4-72.9c-18.7-15.4-45.8-15.4-64.4 0L4.3 251.5c-10.5 8.6-4.4 25.5 9.8 25.5H48V456c0 30.9 25.1 56 56 56H472c30.9 0 56-25.1 56-56V277.1h33.9c14.1 0 20.3-16.9 9.7-25.5z"
            />
        </svg>
        Inicio
    </a>
    <a href="/favoritos" class="header-icon">
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
        Favoritos
    </a>
</header>

<main>
    <RouterView {options} />
</main>
