use open::that as open_in_browser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    App, AppHandle, Manager, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use url::Url;

const CHATGPT_URL: &str = "https://chat.openai.com";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if app.get_webview_window("main").is_none() {
                initialize_application(app)?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Prepares configuration, window, and tray so the app feels desktop-native.
fn initialize_application<R: tauri::Runtime>(app: &App<R>) -> tauri::Result<()> {
    let (config_state, hide_decorations) = load_initial_preferences(app);
    persist_decoration_pref(app.handle(), &config_state, hide_decorations);

    let (decorations, _window) = init_main_window(app, hide_decorations)?;
    let tray_state = build_tray(app, &decorations, &config_state)?;

    // keep tray icon alive for the lifetime of the app
    app.manage(tray_state);
    Ok(())
}

/// Loads persisted preferences, allowing an environment variable to override them.
fn load_initial_preferences<R: tauri::Runtime>(app: &App<R>) -> (Arc<Mutex<AppConfig>>, bool) {
    let mut config = load_app_config(app);
    if let Some(env_override) = decoration_pref_from_env() {
        config.hide_decorations = env_override;
    }

    let hide_decorations = config.hide_decorations;
    (Arc::new(Mutex::new(config)), hide_decorations)
}

/// Parses the decoration override from `CHATGPT_TAURI_HIDE_DECORATIONS`.
fn decoration_pref_from_env() -> Option<bool> {
    std::env::var("CHATGPT_TAURI_HIDE_DECORATIONS")
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
}

/// Creates the main webview window and applies the decoration state.
fn init_main_window<R: tauri::Runtime>(
    app: &App<R>,
    hide_decorations: bool,
) -> tauri::Result<(Arc<Mutex<bool>>, WebviewWindow<R>)> {
    let decorations = Arc::new(Mutex::new(!hide_decorations));
    let cache_dir = prepare_webview_cache(app);

    let mut webview_builder = WebviewWindowBuilder::new(
        app,
        "main",
        WebviewUrl::External(
            CHATGPT_URL
                .parse()
                .expect("the chatgpt url constant should always be valid"),
        ),
    )
    .title("ChatGPT Desktop")
    .theme(Some(Theme::Dark))
    .inner_size(1200.0, 800.0)
    .visible(true)
    .on_new_window(|url, _features| {
        if is_allowed_url(&url) {
            tauri::webview::NewWindowResponse::Allow
        } else {
            let _ = open_in_browser(url.as_str());
            tauri::webview::NewWindowResponse::Deny
        }
    });

    if let Some(dir) = cache_dir {
        webview_builder = webview_builder.data_directory(dir);
    }

    let window = webview_builder.build()?;
    if hide_decorations {
        let _ = window.set_decorations(false);
    }

    Ok((decorations, window))
}

/// Ensures the webview cache directory exists and reports its path.
fn prepare_webview_cache<R: tauri::Runtime>(app: &App<R>) -> Option<PathBuf> {
    app.path().app_data_dir().ok().and_then(|dir| {
        let cache_dir = dir.join("webview-cache");
        match fs::create_dir_all(&cache_dir) {
            Ok(_) => Some(cache_dir),
            Err(err) => {
                eprintln!("Failed to create webview cache directory: {err}");
                None
            }
        }
    })
}

/// Builds the system tray so the user can toggle window chrome or exit quickly.
fn build_tray<R: tauri::Runtime>(
    app: &App<R>,
    decorations: &Arc<Mutex<bool>>,
    config_state: &Arc<Mutex<AppConfig>>,
) -> tauri::Result<TrayState<R>> {
    let toggle_item = MenuItem::with_id(
        app,
        "toggle-window-chrome",
        "Toggle window chrome",
        true,
        Some("CmdOrCtrl+Shift+B"),
    )?;
    let quit_item = MenuItem::with_id(app, "quit-app", "Quit ChatGPT", true, None::<&str>)?;

    let toggle_id = toggle_item.id().clone();
    let quit_id = quit_item.id().clone();

    let menu = MenuBuilder::new(app)
        .item(&toggle_item)
        .item(&quit_item)
        .build()?;

    let mut tray_builder = TrayIconBuilder::new()
        .tooltip("ChatGPT Desktop")
        .menu(&menu)
        .on_menu_event({
            let decorations = Arc::clone(decorations);
            let config_state = Arc::clone(config_state);
            move |handle: &AppHandle<_>, event| {
                let event_id = event.id();
                if event_id == &toggle_id {
                    if let Some(visible) = toggle_window_decorations(handle, &decorations) {
                        persist_decoration_pref(handle, &config_state, !visible);
                    }
                } else if event_id == &quit_id {
                    handle.exit(0);
                }
            }
        });

    if let Some(icon) = select_tray_icon(app) {
        tray_builder = tray_builder.icon(icon);
    }

    Ok(TrayState::new(tray_builder.build(app)?))
}

/// Toggles Tauri window decorations on the main thread and reports the new value.
fn toggle_window_decorations<R: tauri::Runtime>(
    handle: &AppHandle<R>,
    decorations: &Arc<Mutex<bool>>,
) -> Option<bool> {
    let (tx, rx) = mpsc::channel();
    let handle_clone = handle.clone();
    let decorations_clone = Arc::clone(decorations);

    let result = handle.run_on_main_thread(move || {
        let result = if let Some(window) = handle_clone.get_webview_window("main") {
            if let Ok(mut current) = decorations_clone.lock() {
                let new_state = !*current;
                match window.set_decorations(new_state) {
                    Ok(_) => {
                        *current = new_state;
                        Some(new_state)
                    }
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        let _ = tx.send(result);
    });

    if result.is_err() {
        return None;
    }

    rx.recv().ok().flatten()
}

/// Keeps the tray icon alive for the lifetime of the app.
struct TrayState<R: tauri::Runtime>(TrayIcon<R>);

impl<R: tauri::Runtime> TrayState<R> {
    fn new(icon: TrayIcon<R>) -> Self {
        Self(icon)
    }
}

/// Picks the embedded icon, falling back to the default window icon when present.
fn select_tray_icon<R: tauri::Runtime>(app: &App<R>) -> Option<Image<'static>> {
    if let Some(icon) = app
        .default_window_icon()
        .map(|icon| icon.clone().to_owned())
    {
        return Some(icon);
    }

    Image::from_bytes(include_bytes!("../icons/32x32.png")).ok()
}

/// Restricts new webview windows to known ChatGPT hosts, otherwise opens in the browser.
fn is_allowed_url(url: &Url) -> bool {
    match url.scheme() {
        "https" | "http" => match url.host_str() {
            Some(host) => {
                host == "chat.openai.com" || host == "chatgpt.com" || host.ends_with(".openai.com")
            }
            None => true,
        },
        "about" | "data" | "blob" => true,
        _ => true,
    }
}

/// Captures user preferences persisted on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    #[serde(default)]
    hide_decorations: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hide_decorations: false,
        }
    }
}

fn load_app_config<R: tauri::Runtime>(app: &App<R>) -> AppConfig {
    let path = config_path_from_app(app);
    if let Some(path) = path {
        if let Ok(bytes) = fs::read(path) {
            if let Ok(cfg) = serde_json::from_slice::<AppConfig>(&bytes) {
                return cfg;
            }
        }
    }

    AppConfig::default()
}

/// Persists the latest decoration preference and reports failures to stderr.
fn persist_decoration_pref<R: tauri::Runtime>(
    handle: &AppHandle<R>,
    config_state: &Arc<Mutex<AppConfig>>,
    hide: bool,
) {
    if let Ok(mut config) = config_state.lock() {
        config.hide_decorations = hide;
        if let Err(err) = save_app_config(handle, &config) {
            eprintln!("Failed to save config: {err}");
        }
    }
}

fn save_app_config<R: tauri::Runtime>(handle: &AppHandle<R>, config: &AppConfig) -> io::Result<()> {
    if let Some(path) = config_path_from_handle(handle) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_vec_pretty(config)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        fs::write(path, data)?;
    }
    Ok(())
}

fn config_path_from_app<R: tauri::Runtime>(app: &App<R>) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("settings.json"))
}

fn config_path_from_handle<R: tauri::Runtime>(handle: &AppHandle<R>) -> Option<PathBuf> {
    handle
        .path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("settings.json"))
}
