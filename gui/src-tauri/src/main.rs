// CortexIntel native desktop shell (Tauri v2).
//
// Rather than duplicate the command surface, the native app runs the same
// embedded HTTP server the CLI uses and points its WebView at it. One backend,
// one auth/session model, one code path across desktop and browser.
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

const PORT: u16 = 8799;

fn main() {
    // Pin the data directory to the per-user OS app-data location so the desktop
    // app always writes to a path it can create (e.g. macOS
    // ~/Library/Application Support/CortexIntel), avoiding permission failures
    // like an unwritable /var/root/.cortexintel.
    if std::env::var_os("CORTEX_HOME_DIR").is_none() {
        if let Some(d) = dirs::data_dir() {
            std::env::set_var("CORTEX_HOME_DIR", d.join("CortexIntel"));
        }
    }

    // Start the embedded engine server on a background thread. Binding is fast,
    // and the frontend retries /api/ping, so the window can load immediately.
    std::thread::spawn(|| {
        if let Err(e) = cortexintel::serve::serve(PORT, false) {
            eprintln!("embedded server error: {e}");
        }
    });

    tauri::Builder::default()
        .setup(|app| {
            use tauri::{WebviewUrl, WebviewWindowBuilder};
            let url = format!("http://127.0.0.1:{PORT}/");
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse().unwrap()))
                .title("CortexIntel — Intelligence Workspace")
                .inner_size(1440.0, 900.0)
                .min_inner_size(1080.0, 720.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running CortexIntel");
}
