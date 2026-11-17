<h1 align="center">QuaRyst</h1>

<p align="center">
  <img src="assets/logo.png" alt="QuaRyst Logo" width="120">
</p>

<p align="center">
  <b>Ultrafast local full-text search with minimal resource footprint</b>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#usage">Usage</a> •
  <a href="#tech-stack">Tech Stack</a> •
  <a href="#performance">Performance</a> •
  <a href="#roadmap">Roadmap</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Iced-0.14-blue?style=flat&logo=rust&logoColor=white" alt="Iced">
  <img src="https://img.shields.io/badge/License-GPL--3.0-blue.svg" alt="License">
</p>

---

<h2 align="center">🚀 Features</h2>

- **⚡ Blazing Fast**: Sub-10ms search queries across millions of indexed local documents
- **🎨 Windows 11 Fluent UI 2**: Glassmorphic top bar, dark/light theme toggle, rounded geometry, and category color badges
- **📂 Filename & Full-Text Modes**: Toggle instantly between full-text document search and filename-only search
- **💾 Minimal Footprint**: Asynchronous Tokio + Rayon + Mimalloc runtime with minimal idle memory usage
- **📄 88+ Supported Formats**: Pure-Rust native extraction for PDF (`unpdf`), Office/ODF/EPUB (`zip` + XML), RTF, legacy `.doc`/`.xls`/`.ppt`, and 62+ code/text formats
- **🔍 Structural Preview**: Live document preview with **Matches Only** (line-numbered) vs **Full Text** toggle and syntax-highlighted code blocks
- **📋 Cards & Table Views**: Switch between rich snippet cards and a sortable Table view with inline column filters (`2024-01~2024-06`, `1 MB~10 MB`, `pdf docx`)
- **🏷️ Category Presets & Filters**: 1-click presets for **Documents**, **Source Code**, **Data & Logs**, and **Images**, plus extension/size/date filters
- **📊 Dynamic Result Sorting**: Re-order results on the fly by **Relevance Score** (with recency tie-breaker), **Date Modified**, **File Size**, or **File Name**
- **🔄 Live File Watching**: Real-time index updates via `notify` event watcher, plus drag-and-drop folder indexing onto the window
- **🎯 Smart Control**: `.gitignore` rule parsing, custom exclude patterns, and global `Alt+Space` hotkey
- **🌙 Instant Theme Switcher**: 1-click direct header toggle between Dark 🌙 and Light ☀️ modes
- **🔒 Guaranteed Privacy**: 100% local processing with zero network calls and zero telemetry


<h2 align="center">📥 Installation</h2>

<h3 align="center">Prerequisites</h3>

- **Windows**: Windows 10/11
- **macOS**: macOS 10.15+
- **Linux**: Vulkan-compatible drivers & development tools (pkg-config, libfontconfig1-dev)

<h3 align="center">Install via Terminal</h3>

**Windows (PowerShell):**
```powershell
irm https://github.com/SV-stark/FindAll/releases/latest/download/quaryst-installer.ps1 | iex
```

**macOS & Linux:**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/SV-stark/FindAll/releases/latest/download/quaryst-installer.sh | sh
```

Or manually download the pre-built binaries for your platform from the [Releases](https://github.com/SV-stark/FindAll/releases) page.

*Note: The installed application binary is called `quaryst`.*

<h3 align="center">Build from Source</h3>

```bash
# Clone the repository
git clone https://github.com/SV-stark/FindAll.git
cd FindAll

# Build the application
cargo build --release

# Run in development mode
cargo run
```

<h2 align="center">🎮 Usage</h2>

<h3 align="center">First Launch</h3>

1.  **Initial Setup**: Select folders to index on first launch
2.  **Indexing**: The app will scan and index your files (this may take a few minutes for large directories)
3.  **Search**: Press `Alt+Space` (or your custom hotkey) to open the search bar from anywhere

<h3 align="center">Search Syntax</h3>

| Query | Description |
|:---|:---|
| `rust tutorial` | Find documents containing both words (prefix-matched by default) |
| `"exact phrase"` | Find exact phrase matches |
| `"budget review"~5` | Proximity search: words within 5 positions |
| `recieve~` / `recieve~2` | Fuzzy typo-tolerant search (1 or 2 edits) |
| `/inv[0-9]+/` | Inline case-insensitive regex search |
| `rust OR python` | Boolean OR operator |
| `code -python` | Exclude documents with "python" (works with phrases/regex too) |
| `title:api` | Search only in document titles |
| `ext:pdf` | Filter by file extension |
| `path:docs` | Filter by folder path |
| `size:1MB~10MB` | Filter by file size (`>5MB`, `<100KB`, `1MB~10MB`) |
| `date:2024-01~2024-06` | Filter by modified date (`today`, `7d`, `2025`, `2024-01~2024-06`) |

<h3 align="center">Keyboard Shortcuts</h3>

| Shortcut | Action |
|:---|:---|
| `Alt+Space` | Toggle search window (Global Hotkey, configurable in Settings) |
| `Ctrl+F` | Focus search input bar |
| `↑` / `↓` | Navigate search results list with live preview rendering |
| `Enter` | Open selected file in default desktop application |
| `Ctrl+Enter` | Open containing folder in Windows File Explorer |
| `Ctrl+C` | Copy selected file's absolute path to clipboard |
| `Esc` | Close search window |

<h3 align="center">Command-Line Interface</h3>

QuaRyst includes a CLI mode for terminal-based querying and scripting:

```bash
# Basic terminal text search
quaryst --cli "query"

# Output as JSON 
quaryst --cli "query" --json
```

<h3 align="center">App Data & Logs</h3>

QuaRyst automatically manages its index database and rolling background logs (retained up to 30 days) locally in the `com.quaryst` data directory:
- **Windows**: `%AppData%\com.quaryst` or `%LocalAppData%\com.quaryst`
- **macOS/Linux**: `~/.local/share/com.quaryst` or `~/.config/com.quaryst`

<h2 align="center">🏗️ Tech Stack</h2>

<h3 align="center">Core Architecture</h3>

| Component | Technology | Purpose |
|:---|:---|:---|
| **Language** | Rust | Zero-overhead, memory-safe core |
| **GUI** | Iced | High-performance, cross-platform UI framework (<30MB RAM) |
| **Search Engine** | Tantivy | Full-text indexing with BM25 scoring |
| **Document Parsing** | `unpdf` + `zip`/XML/RTF | Pure-Rust, zero-C-dep extraction for **88 formats** |
| **Metadata DB** | redb | Pure Rust key-value storage |
| **Concurrency** | Rayon + Tokio | Parallel processing + async I/O |

<h3 align="center">Supported File Formats (88 Extensions)</h3>

| Category | Supported Formats |
|:---|:---|
| **PDF** | `pdf` (via `unpdf`) |
| **MS Office (OOXML)** | `docx`, `docm`, `dotx`, `dotm`, `xlsx`, `xlsm`, `xltx`, `xltm`, `pptx`, `pptm`, `ppsx`, `potx`, `potm` |
| **MS Office (Legacy)** | `doc`, `xls`, `ppt` (UTF-16LE/ASCII text extraction) |
| **OpenDocument & E-Books** | `odt`, `ods`, `odp`, `epub` |
| **Text, Markup & Geo** | `txt`, `md`, `markdown`, `rtf`, `html`, `htm`, `xhtml`, `xht`, `xml`, `svg`, `kml`, `tex` |
| **Data, Config & Logs** | `json`, `yaml`, `yml`, `toml`, `csv`, `tsv`, `ini`, `conf`, `config`, `env`, `log`, `sql`, `graphql`, `proto`, `diff`, `patch` |
| **Source Code** | `rs`, `py`, `js`, `jsx`, `ts`, `tsx`, `css`, `scss`, `sass`, `less`, `c`, `cpp`, `cc`, `cxx`, `h`, `hpp`, `hh`, `hxx`, `cs`, `go`, `java`, `kt`, `kts`, `swift`, `php`, `rb`, `lua`, `sh`, `bash`, `zsh`, `bat`, `cmd`, `ps1`, `psm1`, `r`, `dart`, `scala`, `zig`, `nim` |

<h2 align="center">📊 Performance</h2>

<div align="center">

QuaRyst uses zero-C-dependency, pure-Rust parsers (`unpdf`, `zip` + streaming XML/RTF strippers, and memory-mapped plaintext decoding) parallelized across Rayon workers at background OS priority.

Benchmarks on standard desktop hardware (e.g., AMD Ryzen 7 / Apple M-series):

</div>

| Metric | Value | Notes |
|:---|:---|:---|
| **Text Extraction** | **Pure Rust + Rayon** | Synchronous parallel extraction via `unpdf` + `zip`/XML + `memmap2` |
| **Indexing Throughput** | **~50-100 MB/s** | Highly parallelized via Rayon & zero-allocation path filtering |
| **Search Latency (p50)** | **< 10ms** | Zero-copy `rkyv` binary indices + lock-free FST lookups |
| **Search Latency (p99)** | **< 35ms** | Optimized Tantivy full-text index with debounced commits |
| **Idle RAM Usage** | **~25-30 MB** | Minimal background footprint securely controlled via Rust |
| **Peak RAM (Indexing)** | **~150-200 MB** | Bounded streaming architecture & smart `blake3` watcher hashing |

<h3 align="center">Comparison</h3>

<div align="center">

| Feature | QuaRyst | AnyTXT | Windows Search | Recoll |
|:---|:---|:---|:---|:---|
| Startup Time | **Instant** | ~2s | System | ~3s |
| Memory Overhead | **~30MB** | ~80MB | ~150MB+ | ~120MB |
| Fast PDF Extraction | ✅ **(`unpdf`)** | ✅ | ⚠️ | ✅ |
| Supported Formats | ✅ **(88)** | ✅ | ❌ | ⚠️ |
| Live Updates | ✅ | ✅ | ✅ | ❌ |
| Cross-Platform | ✅ | ⚠️ | ❌ | ✅ |

</div>

<h2 align="center">🗺️ Roadmap</h2>

### Phase 1: Foundation (Completed ✅)
- [x] High-performance core with **Rust**
- [x] Efficient UI framework with **Iced**
- [x] Basic file parsers for PDF, DOCX, and TXT
- [x] Parallel filesystem scanning and incremental indexing

### Phase 2: Polish & UX (Completed ✅)
- [x] Advanced search filters (date, size, extension)
- [x] Global "Spotlight-style" search bar (Alt+Space)
- [x] Real-time file indexing and system tray integration
- [x] Fast filename-only search mode for instant navigation
- [x] Optimized result preview and metadata retrieval
- [x] **Network Discovery**: Support for enterprise SMB/CIFS and NAS drives

### Phase 3: Advanced Intelligence (In Progress 🚧)
- [x] **Native Text Extraction**: 88 formats supported in pure Rust (`unpdf`, `zip`/XML, RTF, legacy Office, plaintext/code)
- [x] **Pinned Results**: Favorite important files for instant access
- [ ] **Semantic Search**: Vector embeddings for context-aware queries
- [ ] **Natural Language Queries**: "files from last week about taxes"

### Phase 4: Expansion & Connectivity (Future 🔮)
- [ ] **Plugin System**: Community-built parsers and search extensions
- [ ] **Cloud Indexing**: Optional encrypted metadata sync across devices
- [ ] **Developer API**: Headless search service for other applications

<h2 align="center">🤝 Contributing</h2>

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

</div>

<h3 align="center">Development Setup</h3>

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and setup
git clone https://github.com/SV-stark/FindAll.git
cd FindAll

# Run development server
cargo run
```

<h2 align="center">📄 License</h2>

<div align="center">

This project is licensed under the GNU General Public License v3 - see the [LICENSE](LICENSE) file for details.

</div>

<h2 align="center">🙏 Acknowledgments</h2>


QuaRyst is built on the shoulders of giants. We are deeply grateful to the following projects:

- **[unpdf](https://github.com/iyulab/unpdf)** - Fast, pure-Rust PDF text and layout extraction.
- **[Tantivy](https://github.com/quickwit-oss/tantivy)** - The blazing-fast, index-based search engine that powers our core.
- **[Iced](https://iced.rs/)** - A battery-included, type-safe GUI library that keeps our RAM usage minimal.
- **[Redb](https://github.com/cberner/redb)** - A simple, portable, and high-performance embedded key-value store.
- **[Tokio](https://tokio.rs/)** & **[Rayon](https://github.com/rayon-rs/rayon)** - The powerhouses behind our asynchronous I/O and parallel processing.


---

<p align="center">
  Made with ❤️ using Rust
</p>
