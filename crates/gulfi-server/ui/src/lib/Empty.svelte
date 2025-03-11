<script lang="ts">
    export let titulo: string;
    export let motivo: string;
    export let solucion: string;
</script>

<div class="empty-state">
    <h1 class="empty-title">{titulo}</h1>

    <p>{motivo}</p>

    <div>
        <p>{solucion}</p>
    </div>

    <a href="/" class="search-button"> Volver al Inicio </a>
</div>
