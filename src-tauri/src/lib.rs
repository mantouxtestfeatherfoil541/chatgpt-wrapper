use open::that as open_in_browser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{
    webview::DownloadEvent, App, AppHandle, Manager, Theme, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_plugin_notification::NotificationExt;
use url::Url;

const CHATGPT_URL: &str = "https://chatgpt.com";

const INIT_SCRIPT: &str = r#"
(function() {
    // Performance: Preconnect to CDN domains
    const preconnectDomains = [
        'https://cdn.oaistatic.com',
        'https://cdn.openai.com'
    ];
    
    preconnectDomains.forEach(domain => {
        const link = document.createElement('link');
        link.rel = 'preconnect';
        link.href = domain;
        link.crossOrigin = 'anonymous';
        document.head?.appendChild(link);
    });

    // Font smoothing
    const style = document.createElement('style');
    style.textContent = `
        * {
            -webkit-font-smoothing: antialiased;
            -moz-osx-font-smoothing: grayscale;
        }
    `;
    if (document.head) {
        document.head.appendChild(style);
    } else {
        document.addEventListener('DOMContentLoaded', function() {
            document.head.appendChild(style);
        });
    }

    // Auto-grant notification permission
    if ('Notification' in window && Notification.permission === 'default') {
        Notification.requestPermission();
    }

    // Reload handler
    document.addEventListener('keydown', function(e) {
        if (e.key === 'F5' || ((e.ctrlKey || e.metaKey) && e.key === 'r')) {
            e.preventDefault();
            window.location.reload();
        }
    }, true);

    // External link handler
    document.addEventListener('click', function(e) {
        const link = e.target.closest('a');
        if (!link) return;
        
        const href = link.href;
        if (!href) return;
        
        try {
            const url = new URL(href);
            const currentOrigin = window.location.origin;
            
            const allowedDomains = [
                'chatgpt.com',
                'chat.openai.com',
                'openai.com',
                'oaistatic.com',
                'oaiusercontent.com'
            ];
            
            const isAllowed = allowedDomains.some(domain => 
                url.hostname === domain || url.hostname.endsWith('.' + domain)
            );
            
            if (!isAllowed && url.origin !== currentOrigin) {
                e.preventDefault();
                e.stopPropagation();
                window.open(href, '_blank');
            }
        } catch (err) {
            console.log(err)
        }
    }, true);
})();
"#;

#[tauri::command]
fn reload_webview(window: WebviewWindow) -> Result<(), String> {
    window
        .eval("window.location.reload();")
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![reload_webview])
        .setup(|app| {
            if app.get_webview_window("main").is_none() {
                initialize_application(app)?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Prepares configuration and window so the app feels desktop-native.
fn initialize_application<R: tauri::Runtime>(app: &App<R>) -> tauri::Result<()> {
    let (config_state, hide_decorations) = load_initial_preferences(app);
    persist_decoration_pref(app.handle(), &config_state, hide_decorations);

    let (_decorations, _window) = init_main_window(app, hide_decorations)?;
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

/// Handles download events: saves to Downloads folder and notifies user.
fn create_download_handler<R: tauri::Runtime>(
    app_handle: AppHandle<R>,
) -> impl Fn(tauri::Webview<R>, DownloadEvent) -> bool {
    let download_path = Arc::new(Mutex::new(Option::<PathBuf>::None));
    
    move |_webview, event| {
        match event {
            DownloadEvent::Requested { destination, .. } => {
                // Get downloads directory
                let download_dir = match app_handle.path().download_dir() {
                    Ok(dir) => dir,
                    Err(_) => return false,
                };
                
                // Set destination to downloads folder
                let final_path = download_dir.join(&destination);
                let mut locked_path = download_path.lock().unwrap();
                *locked_path = Some(final_path.clone());
                *destination = final_path;
                
                // Show notification
                let app = app_handle.clone();
                let filename = destination.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file")
                    .to_string();
                
                tauri::async_runtime::spawn(async move {
                    let _ = app.notification()
                        .builder()
                        .title("Downloading file")
                        .body(&format!("Saving: {}", filename))
                        .show();
                });
                
                return true;
            }
            DownloadEvent::Finished { success, .. } => {
                let path_opt = download_path.lock().unwrap().clone();
                
                if let Some(final_path) = path_opt {
                    let app = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        if success {
                            let _ = app.notification()
                                .builder()
                                .title("Download completed")
                                .body(&format!("Saved to: {}", final_path.display()))
                                .show();
                        } else {
                            let _ = app.notification()
                                .builder()
                                .title("Download failed")
                                .body("Could not complete the download")
                                .show();
                        }
                    });
                }
                return true;
            }
            _ => {}
        }
        true
    }
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
    .min_inner_size(400.0, 300.0)
    .visible(true)
    .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
    .accept_first_mouse(true)
    .initialization_script(INIT_SCRIPT)
    .additional_browser_args("--enable-features=WebRTCPipeWireCapturer,VaapiVideoDecodeLinuxGL --enable-gpu-rasterization --enable-zero-copy --disable-software-rasterizer --enable-accelerated-video-decode")
    .on_download(create_download_handler(app.handle().clone()))
    .on_new_window(|url, _features| {
        if url.scheme() == "blob" || url.scheme() == "data" {
            return tauri::webview::NewWindowResponse::Deny;
        }
        
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

/// Restricts new webview windows to known ChatGPT hosts, otherwise opens in the browser.
fn is_allowed_url(url: &Url) -> bool {
    match url.scheme() {
        "https" | "http" => match url.host_str() {
            Some(host) => {
                host == "chatgpt.com" 
                || host == "chat.openai.com" 
                || host.ends_with(".openai.com")
                || host.ends_with(".oaistatic.com")
                || host.ends_with(".oaiusercontent.com")
            }
            None => true,
        },
        "about" | "data" | "blob" | "wss" | "ws" => true,
        _ => false,
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
