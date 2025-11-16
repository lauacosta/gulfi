<script lang="ts">
import { RouterView } from "@dvcol/svelte-simple-router/components";
import type { Route, RouterOptions } from "@dvcol/svelte-simple-router/models";
import Layout from "./lib/Layout.svelte";
import Sidebar from "./lib/Sidebar.svelte";
import Fallback from "./routes/Fallback.svelte";
import Favorites from "./routes/Favorites.svelte";
import History from "./routes/History.svelte";
import Home from "./routes/Home.svelte";

const RouteName = {
	Home: "home",
	Favorites: "favorites",
	History: "history",
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
		name: RouteName.Favorites,
		path: "/favorites",
		component: Favorites,
	},

	{
		name: RouteName.History,
		path: "/history",
		component: History,
	},

	{
		name: RouteName.Fallback,
		path: "*",
		component: Fallback,
	},
] as const;

export const options: RouterOptions<RouteNames> = { routes } as const;
</script>

<Layout>
    <Sidebar />
    <RouterView {options} />
</Layout>
