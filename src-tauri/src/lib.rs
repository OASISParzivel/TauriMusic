mod lyrics;
mod model;
mod scanner;

use model::Library;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct AppState {
    pub lib: Mutex<Library>,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

const LIBRARY_FILE: &str = "library.json";

#[tauri::command]
fn get_library(state: State<AppState>) -> Library {
    state.lib.lock().map(|l| l.clone()).unwrap_or_default()
}

#[tauri::command]
fn add_folder(path: String, state: State<AppState>) -> Result<(), String> {
    if !Path::new(&path).is_dir() {
        return Err("该路径不是一个文件夹".into());
    }
    let mut lib = state.lib.lock().map_err(|_| "内部状态不可用".to_string())?;
    if !lib.folders.iter().any(|f| f == &path) {
        lib.folders.push(path);
        lib.save(&state.data_dir.join(LIBRARY_FILE))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn remove_folder(path: String, state: State<AppState>) -> Result<(), String> {
    let mut lib = state.lib.lock().map_err(|_| "内部状态不可用".to_string())?;
    lib.folders.retain(|f| f != &path);
    lib.tracks.retain(|t| !t.path.starts_with(&path));
    lib.save(&state.data_dir.join(LIBRARY_FILE))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn pick_music_folder() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("选择音乐文件夹")
        .pick_folder()
        .await
        .map(|f| f.path().to_string_lossy().to_string())
}

#[tauri::command]
async fn scan_library(app: tauri::AppHandle) -> Result<model::ScanReport, String> {
    tauri::async_runtime::spawn_blocking(move || scanner::run_scan(&app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn get_lyrics(id: String, state: State<AppState>) -> Option<lyrics::LyricsPayload> {
    let lib = state.lib.lock().ok()?;
    lyrics::find_in_library(&id, &lib)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();
            let data_dir = handle
                .path()
                .app_data_dir()
                .expect("无法确定应用数据目录");
            let cache_dir = handle
                .path()
                .app_cache_dir()
                .expect("无法确定应用缓存目录");
            std::fs::create_dir_all(&data_dir).ok();
            std::fs::create_dir_all(cache_dir.join("covers")).ok();
            let lib = Library::load(&data_dir.join(LIBRARY_FILE));
            app.manage(AppState {
                lib: Mutex::new(lib),
                data_dir,
                cache_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_library,
            add_folder,
            remove_folder,
            pick_music_folder,
            scan_library,
            get_lyrics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
