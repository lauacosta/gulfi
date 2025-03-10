<script lang="ts">
    import { writable } from "svelte/store";

    let isExpanded = $state(false);

    let currentPath = writable(window.location.pathname);

    window.addEventListener("popstate", () => {
        currentPath.set(window.location.pathname);
    });

    function isActive(path: string) {
        return $currentPath === path;
    }
</script>

<aside class={isExpanded ? "expanded" : "collapsed"}>
    <button class="toggle-btn" onclick={() => (isExpanded = !isExpanded)}>
        {#if isExpanded}
            <svg
                xmlns="http://www.w3.org/2000/svg"
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="lucide lucide-panel-left-close"
                ><rect width="18" height="18" x="3" y="3" rx="2" /><path
                    d="M9 3v18"
                /><path d="m16 15-3-3 3-3" /></svg
            >
        {:else}
            <svg
                xmlns="http://www.w3.org/2000/svg"
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="lucide lucide-panel-left-open"
                ><rect width="18" height="18" x="3" y="3" rx="2" /><path
                    d="M9 3v18"
                /><path d="m14 9 3 3-3 3" /></svg
            >
        {/if}
    </button>

    <nav>
        <ul>
            <li class={isActive("/") ? "active" : ""}>
                <a href="/" title="Busqueda">
                    <span class="icon">
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="24"
                            height="24"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            class="lucide lucide-search"
                            ><circle cx="11" cy="11" r="8" /><path
                                d="m21 21-4.3-4.3"
                            /></svg
                        >
                    </span>
                    {#if isExpanded}
                        <span class="label">Busqueda</span>
                    {/if}
                </a>
            </li>
            <li class={isActive("/favoritos") ? "active" : ""}>
                <a href="/favoritos" title="Favoritos">
                    <span class="icon"
                        ><svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="24"
                            height="24"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            class="lucide lucide-star"
                            ><path
                                d="M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z"
                            /></svg
                        ></span
                    >
                    {#if isExpanded}
                        <span class="label">Favoritos</span>
                    {/if}
                </a>
            </li>

            <li class={isActive("/historial") ? "active" : ""}>
                <a href="/historial" title="Historial">
                    <span class="icon"
                        ><svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="24"
                            height="24"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            class="lucide lucide-history"
                            ><path
                                d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"
                            /><path d="M3 3v5h5" /><path d="M12 7v5l4 2" /></svg
                        ></span
                    >
                    {#if isExpanded}
                        <span class="label">Historial</span>
                    {/if}
                </a>
            </li>
        </ul>
    </nav>
</aside>
