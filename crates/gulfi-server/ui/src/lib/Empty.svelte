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

<style>

    
.empty-state {
	text-align: center;
	padding: 40px 0;
	color: #6c757d;
	max-width: 500px;
	margin: auto;
	background: #f9f9f9;
	border-radius: 10px;
	box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
	position: absolute;
	top: 50%;
	left: 50%;
	transform: translate(-50%, -50%);
}

.empty-state h1 {
	color: #333;
	font-size: 1.8rem;
	padding: 1rem;
	margin-bottom: 1rem;
}

.empty-state p {
	color: #666;
	font-size: 1rem;
	padding: 0.5rem;
	line-height: 1.5;
}

.empty-state div {
	margin: 1.5rem 0;
	display: flex;
	justify-content: center;
}

.empty-state a {
	display: inline-block;
	padding: 10px 20px;
	background: var(--my-green);
	color: white;
	font-weight: bold;
	text-decoration: none;
	border-radius: 5px;
	transition: background 0.3s ease-in-out;
}
</style>
