# Gulfi 🔍

<div align="center">

[![CircleCI](https://dl.circleci.com/status-badge/img/circleci/HVk4cDAtMKJw9W8KJXwwZC/YDFhpnAeGZetGyePNFt7ZC/tree/main.svg?style=svg)](https://dl.circleci.com/status-badge/redirect/circleci/HVk4cDAtMKJw9W8KJXwwZC/YDFhpnAeGZetGyePNFt7ZC/tree/main) ![Status: Beta](https://img.shields.io/badge/status-beta%20but%20usable-brightgreen)

**Lightweight web search server backed by SQLite**

[Features](#features) • [Installation](#installation) • [Usage](#usage)

</div>

---

> ⚠️ **Beta**  
> Gulfi is under active development. APIs and behavior may change.

## Overview

Gulfi is a small, self-hosted web server that provides search capabilities over structured data stored in SQLite.  
It is designed to be simple to deploy, fast to query, and easy to integrate into existing systems.

At its core, Gulfi focuses on **exact and hybrid search over local datasets**, with a future-facing design that will support a dedicated search DSL.

## Features

- HTTP API for querying structured data
- SQLite-backed storage
- Full-text search via `fts5`
- Single static binary
- Designed for local and self-hosted usage

## Installation

### Prerequisites

- Rust 1.78.0+
- Git

### Build from source

```bash
git clone https://github.com/lauacosta/gulfi.git
cd gulfi
cargo build --release
