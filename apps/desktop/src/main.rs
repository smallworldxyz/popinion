// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  // WebKitGTK's DMABUF renderer (2.40+) fails EGL init on many Linux GPU/driver
  // and Wayland setups ("Could not create default EGL display: EGL_BAD_PARAMETER"
  // → abort). Disable it so the webview falls back to a renderer that works
  // everywhere; for a web UI the perf cost is negligible. Respect an explicit
  // override so advanced users can re-enable it.
  #[cfg(target_os = "linux")]
  if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
  }

  app_lib::run();
}
