<script lang="ts">
import Sidebar from "./Sidebar.svelte";
</script>

<div class="app-layout">
    <aside class="sidebar">
        <Sidebar />
    </aside>
    <div class="main-section">
        <main class="content-area">
            <slot />
        </main>
    </div>
    <aside class="sidebar">
        <Sidebar />
    </aside>
</div>

<style>
.app-layout {
    display: flex;
    height: 100vh;
    background-color: #1e1e1e;
    color: #eaeaea;
}

.sidebar {
    width: 240px;
    background-color: #181818;
    border-right: 1px solid #2a2a2a;
    display: flex;
    flex-direction: column;
}

.main-section {
    flex: 1;
    display: flex;
    flex-direction: column;
}

.top-bar {
    background-color: #181818;
    padding: 1rem;
    border-bottom: 1px solid #2a2a2a;
}

.content-area {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem;
    background-color: #1e1e1e;
}
</style>
