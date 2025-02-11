---
<div align = "center">

# Gulfi 🔍

<a href=https://github.com/lauacosta/gulfi/actions/workflows/general.yaml>
    <img src=https://github.com/lauacosta/gulfi/actions/workflows/general.yaml/badge.svg>
</a>

Gulfi es una herramienta para búsquedas exactas, semánticas e híbridas sobre datos en una base de datos SQLite.

</div>

---

> [!WARNING]
> Este proyecto se encuentra en desarrollo y no está terminado. Puede contener bugs o funcionalidades incompletas.

## Features
El proyecto utiliza las extensiones de sqlite [fts5](https://sqlite.org/fts5.html) y [sqlite-vec](https://github.com/asg017/sqlite-vec).

- Búsqueda Exacta: Realizar búsquedas estrictas de acuerdo al query.
- Búsqueda Semántica: Indentifica datoso similares usando modelos de IA.
- Búsqueda Híbrida: Diferentes combinaciones de búsqueda exacta y semántica:
    - Re-rank by Semantics: Realiza una búsqueda exacta y los re-ordena de acuerdo a su distancia vectorial con respecto al query.
    - Reciprocal Rank Fusion: Valora los resultados obtenidos por ambos métodos por sobre los demás.
    - Keyword First: Devuelve los resultados exactos primeros y luego los semánticos.

Recomiendo leer el blog de [Alex Garcia](https://alexgarcia.xyz/blog/2024/sqlite-vec-hybrid-search/index.html#which-should-i-choose) para tener una idea de en qué casos cada método es más conveniente.

## Build
> [!IMPORTANT]
> Para compilar la aplicación asegurate de tener una version de rustc +1.78.0.
```
$ git clone https://github.com/lauacosta/gulfi.git
$ cd gulfi
$ cargo build --release
```

## Usage
Para ver un resumen de las funciones, puedes ejecutar `gulfi --help`:
```
 _____       _  __ _
|  __ \     | |/ _(_)
| |  \/_   _| | |_ _
| | __| | | | |  _| |
| |_\ \ |_| | | | | |
 \____/\__,_|_|_| |_| 1.1.0

    @lauacosta/gulfi


Usage: gulfi [OPTIONS] <COMMAND>

Commands:
  serve  Inicia el servidor HTTP y expone la interfaz web
  sync   Actualiza la base de datos
  help   Print this message or the help of the given subcommand(s)

Options:
      --log-level <LOGLEVEL>  [default: INFO]
  -h, --help                  Print help
  -V, --version               Print version
```

