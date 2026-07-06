# CortexIntel — Desktop GUI (Tauri + WebView)

A macOS desktop workspace for the CortexIntel engine, implementing the
`DESIGN.md` interface: sidebar navigation, an interactive graph workspace,
contextual entity panel, dashboard, timeline, reports, command palette and a
run/import modal — dark intelligence theme.

The GUI is a thin **Tauri v2** shell: the Rust backend (`src-tauri/`) exposes the
same `cortexintel` engine used by the CLI through Tauri commands, and the WebView
frontend (`dist/`) renders everything. No separate server, no bundler — the
frontend is plain HTML/CSS/JS.

## Architecture

```
gui/
  dist/            # WebView frontend (static, no build step)
    index.html     # layout: topbar, sidebar, graph, context panel, modals
    styles.css     # dark intelligence theme (+ light fallback)
    app.js         # canvas force-directed graph, Tauri bridge, views
  src-tauri/       # Rust backend
    src/main.rs    # #[tauri::command] wrappers over cortexintel::api
    tauri.conf.json
    capabilities/  # window ACL
    icons/         # generated app icons
```

Tauri commands (all forward to `cortexintel::api`):

| Command | Purpose |
|---------|---------|
| `list_domains` | verticals for the menu |
| `list_data_types` | data-type options |
| `list_agents` | agent catalog for a vertical |
| `doctor` | Claude/Codex/mock health |
| `run_analysis` | run the full pipeline, return consolidated JSON |
| `load_graph` | load a previously written `graph.json` |

`run_analysis` runs on a blocking thread so the window stays responsive while
the LLM agents execute.

## Run it

Requires Node (for the Tauri CLI dev server helper) and the Tauri CLI:

```bash
cargo install tauri-cli --version "^2.0"   # once

cd gui/src-tauri
cargo tauri dev      # launch the desktop app (dev)
cargo tauri build    # produce a .app / .dmg in target/release/bundle
```

The frontend also opens **standalone** in any browser (`dist/index.html`): when
it can't find the Tauri bridge it loads an embedded sample graph, so the design
is fully explorable without launching the app.

## Notes

- Running an analysis from the GUI drives the same engine as the CLI, including
  the `IS_SANDBOX=1` handling that lets Claude run under root.
- Keyboard: `⌘K` command palette · `⌘R` run · `⌘F` filter graph · `Esc` close.
- Graph interactions: drag nodes, scroll to zoom, drag canvas to pan, click a
  node for details, right-click for the context menu.
