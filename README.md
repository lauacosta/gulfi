# Gulfi 🔍

<div align="center">

[![CircleCI](https://dl.circleci.com/status-badge/img/circleci/HVk4cDAtMKJw9W8KJXwwZC/YDFhpnAeGZetGyePNFt7ZC/tree/main.svg?style=svg)](https://dl.circleci.com/status-badge/redirect/circleci/HVk4cDAtMKJw9W8KJXwwZC/YDFhpnAeGZetGyePNFt7ZC/tree/main)

**Search tool for exact, semantic, and hybrid searches over CSV and JSON files**

[Features](#features) • [Installation](#installation) • [Usage](#usage) • [Search Methods](#search-methods)

</div>

---

> [!WARNING]
> This project is currently under development and not yet complete. It may contain bugs or incomplete functionality.

## Overview

Gulfi is a comprehensive search solution built on SQLite that combines traditional exact search with modern semantic search capabilities. By leveraging SQLite extensions [fts5](https://sqlite.org/fts5.html) and [sqlite-vec](https://github.com/asg017/sqlite-vec), Gulfi provides flexible and powerful search functionality for your data.

## Features

### 🎯 **Exact Search**
Perform strict searches that match your query precisely using SQLite's full-text search capabilities.

### 🧠 **Semantic Search**
Identify similar data using AI models that understand the meaning and context behind your queries.

### 🔀 **Hybrid Search**
Combine the best of both worlds with multiple hybrid search strategies:

- **Reciprocal Rank Fusion**: Merges and ranks results from both exact and semantic searches using fusion algorithms

For a detailed comparison of when to use each method, check out [Alex Garcia's excellent blog post](https://alexgarcia.xyz/blog/2024/sqlite-vec-hybrid-search/index.html#which-should-i-choose) on hybrid search strategies.

## Installation

### Prerequisites

- Rust compiler version 1.78.0 or higher
- Git

### Build from Source

```bash
# Clone the repository
git clone https://github.com/lauacosta/gulfi.git

# Navigate to the project directory
cd gulfi

# Build the release version
cargo build --release
```

The compiled binary will be available at `target/release/gulfi`.

## Usage

### Quick Start

To see all available commands and options:

```bash
gulfi --help
```

``` txt
 _____       _  __ _
|  __ \     | |/ _(_)
| |  \/_   _| | |_ _
| | __| | | | |  _| |
| |_\ \ |_| | | | | |
 \____/\__,_|_|_| |_| 1.2.0

    @lauacosta/gulfi


Usage: gulfi [OPTIONS] [COMMAND]

Commands:
  serve        Starts the HTTP server
  sync         Updates the database
  list         Lists all defined documents
  add          Adds a new document
  delete       Deletes a document
  create-user  Creates a new user in the database
  help         Print this message or the help of the given subcommand(s)

Options:
      --level <LOGLEVEL>    [default: INFO]
      --database-path <DB>  Path to the sqlite database [default: ./gulfi.db]
  -h, --help                Print help
  -V, --version             Print version
```

### Commands

#### Starting the Web Server

Launch the HTTP server with the web interface:

```bash
gulfi serve --help

# Starts the HTTP server
# 
# Usage: gulfi serve [OPTIONS]
# 
# Options:
#   -I, --interface <INTERFACE>  Sets the IP address [default: 127.0.0.1]
#   -P, --port <PORT>            Sets the port [default: 3000]
#       --open                   Opens the web interface
#   -h, --help                   Print help

gulfi serve --open
```

This will start a local web server and automatically open a user-friendly interface in the browser if you have a default enabled.
If running on a headless server (like a VPS), omit --open and access the interface manually via http://<host>:<port>.

#### Syncing the Database

Update your database with new data:

```bash
gulfi sync --help

# Updates the database
# 
# Usage: gulfi sync [OPTIONS] <DOCUMENT> [SYNC_STRAT]
# 
# Arguments:
#   <DOCUMENT>
#   [SYNC_STRAT]  Sets the strategy for updating [default: fts] [possible values: fts, vector, all]
# 
# Options:
#       --force                    Updates from scratch
#       --base-delay <BASE_DELAY>  Sets the base time for backoff in requests in ms [default: 2]
#       --chunk-size <CHUNK_SIZE>  Sets the size of the chunks when splitting the entries for processing [default: 1024]
#   -h, --help                     Print help
```

The sync command updates the database entries for a given document. A document is a dataset definition that you've previously added via gulfi add.

> 📝 Tip: You can use --force to rebuild all indexes from scratch if your data has changed significantly.

### Showing available documents
```bash
gulfi list --help

# Lists all defined documents
# 
# Usage: gulfi list
# 
# Options:
#   -h, --help  Print help
```

### Adding a new document
```bash
gulfi add --help

# Adds a new document
# 
# Usage: gulfi add
# 
# Options:
#   -h, --help  Print help
```

This command launches an interactive setup to help define and register a new document for indexing.

### Deleting a document
```bash
gulfi delete --help

# Deletes a document
# 
# Usage: gulfi delete <DOCUMENT>
# 
# Arguments:
#   <DOCUMENT>
# 
# Options:
#   -h, --help  Print help
```

### Creating a new user

> ⚠️ Note: User authentication is a planned feature. For now, this command prepares the groundwork for future secure access and personalization.

```bash
gulfi create-user --help
# Creates a new user in the database
# 
# Usage: gulfi create-user <USERNAME> <PASSWORD>
# 
# Arguments:
#   <USERNAME>
#   <PASSWORD>
# 
# Options:
#   -h, --help  Print help
```

### Configuration

You can adjust the logging level to control the verbosity of output:

```bash
# Set to DEBUG for detailed logs
gulfi --log-level DEBUG serve

# Set to ERROR for minimal output
gulfi --log-level ERROR serve
```

## Search Methods

### When to Use Each Method

| Method | Best For | Use Case Example |
|--------|----------|------------------|
| **Exact Search** | Precise matches, technical terms, specific phrases | Finding exact product codes, specific names |
| **Semantic Search** | Conceptual similarity, related topics | Finding documents about similar concepts |
| **Reciprocal Rank Fusion** | Balanced results from both approaches | General-purpose search applications |


For a detailed comparison of when to use each method, check out [Alex Garcia's excellent blog post](https://alexgarcia.xyz/blog/2024/sqlite-vec-hybrid-search/index.html#which-should-i-choose) on hybrid search strategies.

## Development Status

This project is actively being developed.

### Roadmap
- [ ] Simplified type system with better design patterns
- [ ] Reduced and optimized web client code
- [ ] Enhanced documentation and examples
- [ ] Performance optimizations
- [ ] Support more search algorithms. Eg. Re-rank by Semantics, Keyword First.
- [ ] Add correct database migrations.
- [ ] Add support for batch jobs.

## License

## 📄 License

GPL-3.0 License - see [LICENSE](LICENSE) file for details.
