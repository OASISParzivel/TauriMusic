mod lyrics;
mod model;
mod scanner;

use model::Library;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct AppState {
    pub lib: Mutex<Library>,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

const LIBRARY_FILE: &str = "library.json";
/// 应用自带的音乐导入目录(data_dir/Music),随应用首次启动创建并登记
const IMPORT_DIR_NAME: &str = "Music";

/// 确保导入目录存在,并把它登记进曲库扫描来源
fn ensure_import_dir(state: &AppState) -> Result<PathBuf, String> {
    let dir = state.data_dir.join(IMPORT_DIR_NAME);
    fs::create_dir_all(&dir).map_err(|e| format!("创建导入目录失败: {e}"))?;
    let mut lib = state.lib.lock().map_err(|_| "内部状态不可用".to_string())?;
    let path = dir.to_string_lossy().to_string();
    if !lib.folders.iter().any(|f| f == &path) {
        lib.folders.push(path);
        lib.save(&state.data_dir.join(LIBRARY_FILE))
            .map_err(|e| e.to_string())?;
    }
    Ok(dir)
}

#[tauri::command]
fn get_import_dir(state: State<AppState>) -> Result<String, String> {
    ensure_import_dir(&state).map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
fn open_import_dir(state: State<AppState>) -> Result<String, String> {
    let dir = ensure_import_dir(&state)?;
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map_err(|e| format!("打开资源管理器失败: {e}"))?;
    Ok(dir.to_string_lossy().into_owned())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    folders_added: u32,
    files_copied: u32,
    skipped: u32,
}

/// 拖拽导入:整个文件夹登记为扫描来源(不复制),散装音频/歌词文件复制进导入目录
#[tauri::command]
fn import_paths(paths: Vec<String>, state: State<AppState>) -> Result<ImportReport, String> {
    let import_dir = ensure_import_dir(&state)?;
    let mut report = ImportReport {
        folders_added: 0,
        files_copied: 0,
        skipped: 0,
    };
    let mut lib = state.lib.lock().map_err(|_| "内部状态不可用".to_string())?;
    for raw in &paths {
        let path = Path::new(raw);
        if path.is_dir() {
            let dir_str = path.to_string_lossy().to_string();
            if !lib.folders.iter().any(|f| f == &dir_str) {
                lib.folders.push(dir_str);
                report.folders_added += 1;
            }
        } else if path.is_file() {
            match copy_into_import_dir(path, &import_dir) {
                Ok(true) => report.files_copied += 1,
                Ok(false) => report.skipped += 1,
                Err(_) => report.skipped += 1,
            }
        } else {
            report.skipped += 1;
        }
    }
    lib.save(&state.data_dir.join(LIBRARY_FILE))
        .map_err(|e| e.to_string())?;
    Ok(report)
}

/// 把单个文件复制进导入目录;只收音频与 .lrc 歌词,同名同大小视为已存在
fn copy_into_import_dir(src: &Path, import_dir: &Path) -> Result<bool, String> {
    let supported = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let e = e.to_ascii_lowercase();
            scanner::EXTENSIONS.contains(&e.as_str()) || e == "lrc"
        })
        .unwrap_or(false);
    if !supported {
        return Ok(false);
    }
    let Some(name) = src.file_name() else {
        return Ok(false);
    };
    let dest = import_dir.join(name);
    if dest.exists() {
        let same = match (fs::metadata(src), fs::metadata(&dest)) {
            (Ok(a), Ok(b)) => a.len() == b.len(),
            _ => false,
        };
        if same {
            return Ok(false);
        }
    }
    fs::copy(src, &dest)
        .map(|_| true)
        .map_err(|e| format!("复制 {} 失败: {e}", src.display()))
}

// ===== TMC 音乐包(.tmc = 标准 7z:音频 + 同名 .lrc 歌词 + cover 封面 + meta.json) =====

fn sanitize_name(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => out.push('_'),
            c if (c as u32) < 32 => {}
            c => out.push(c),
        }
    }
    let t = out.trim().to_string();
    if t.is_empty() { "track".into() } else { t }
}

/// 解包 .tmc 到导入目录下的同名子目录,由扫描器拾取(meta.json 会补齐缺失标签)
#[tauri::command]
fn import_tmc(path: String, state: State<AppState>) -> Result<String, String> {
    let src = PathBuf::from(&path);
    if !src.is_file() {
        return Err("文件不存在".into());
    }
    let import_dir = ensure_import_dir(&state)?;
    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "tmc".into());
    let dest = import_dir.join(&stem);
    if dest.exists() {
        fs::remove_dir_all(&dest).map_err(|e| format!("清理旧目录失败: {e}"))?;
    }
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    sevenz_rust::decompress_file(&src, &dest).map_err(|e| format!("解包失败: {e}"))?;
    Ok(dest.to_string_lossy().to_string())
}

/// 把曲库中的一首歌打包导出为 .tmc
#[tauri::command]
async fn export_tmc(id: String, dest: String, app: tauri::AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || export_tmc_blocking(&id, &dest, &app))
        .await
        .map_err(|e| e.to_string())?
}

fn export_tmc_blocking(id: &str, dest: &str, app: &tauri::AppHandle) -> Result<String, String> {
    use sevenz_rust::{SevenZArchiveEntry, SevenZWriter};

    let state = app.state::<AppState>();
    let track = {
        let lib = state.lib.lock().map_err(|_| "内部状态不可用".to_string())?;
        lib.tracks.iter().find(|t| t.id == id).cloned()
    };
    let Some(t) = track else {
        return Err("曲目不存在".into());
    };
    let audio = Path::new(&t.path);
    if !audio.is_file() {
        return Err(format!("音频文件不存在: {}", t.path));
    }
    let ext = audio
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase().to_string())
        .unwrap_or_else(|| "mp3".into());
    let stem = sanitize_name(&t.title);

    // meta.json 先落盘,再随音频/歌词/封面一起打包
    let tmp = std::env::temp_dir().join(format!("taurimusic-export-{}", std::process::id()));
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let meta_path = tmp.join("meta.json");
    let meta = serde_json::json!({
        "title": t.title,
        "artist": t.artist,
        "album": t.album,
        "albumArtist": t.album_artist,
        "year": t.year,
        "trackNo": t.track_no,
        "genre": t.genre,
    });
    fs::write(&meta_path, serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    let mut dest_path = PathBuf::from(dest);
    if dest_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase() != "tmc")
        .unwrap_or(true)
    {
        dest_path.set_extension("tmc");
    }
    if let Some(parent) = dest_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut writer = SevenZWriter::create(&dest_path).map_err(|e| format!("创建 TMC 失败: {e}"))?;
    writer
        .push_archive_entry(
            SevenZArchiveEntry::from_path(audio, format!("{stem}.{ext}")),
            fs::File::open(audio).ok(),
        )
        .map_err(|e| e.to_string())?;
    if let Some(lrc) = &t.lrc_path {
        let lrc_path = Path::new(lrc);
        if lrc_path.is_file() {
            writer
                .push_archive_entry(
                    SevenZArchiveEntry::from_path(lrc_path, format!("{stem}.lrc")),
                    fs::File::open(lrc_path).ok(),
                )
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(cover) = &t.cover {
        let cover_path = Path::new(cover);
        if cover_path.is_file() {
            let cext = cover_path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase().to_string())
                .unwrap_or_else(|| "jpg".into());
            let cname = if cext == "png" { "cover.png" } else { "cover.jpg" };
            writer
                .push_archive_entry(
                    SevenZArchiveEntry::from_path(cover_path, cname.to_string()),
                    fs::File::open(cover_path).ok(),
                )
                .map_err(|e| e.to_string())?;
        }
    }
    writer
        .push_archive_entry(
            SevenZArchiveEntry::from_path(&meta_path, "meta.json".to_string()),
            fs::File::open(&meta_path).ok(),
        )
        .map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| format!("写入 TMC 失败: {e}"))?;
    let _ = fs::remove_file(&meta_path);
    Ok(dest_path.to_string_lossy().to_string())
}

#[tauri::command]
async fn pick_tmc_file() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("选择 TMC 音乐包")
        .add_filter("TMC 音乐包 (*.tmc)", &["tmc"])
        .pick_file()
        .await
        .map(|f| f.path().to_string_lossy().to_string())
}

#[tauri::command]
async fn pick_tmc_dest(default_name: String) -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("导出 TMC 音乐包")
        .add_filter("TMC 音乐包 (*.tmc)", &["tmc"])
        .set_file_name(&format!("{default_name}.tmc"))
        .save_file()
        .await
        .map(|f| f.path().to_string_lossy().to_string())
}

// ===== 在线元数据补全(网易云公开接口,仅补封面/歌词,不涉及流媒体) =====

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeteaseReport {
    cover: bool,
    lyrics: u32,
    skipped: u32,
}

/// 为整张专辑在线匹配封面与歌词:封面存为曲目目录 cover.jpg,歌词存为同名 .lrc
#[tauri::command]
async fn netease_enrich_album(
    album_key: String,
    app: tauri::AppHandle,
) -> Result<NeteaseReport, String> {
    tauri::async_runtime::spawn_blocking(move || netease_enrich_blocking(&album_key, &app))
        .await
        .map_err(|e| e.to_string())?
}

fn netease_enrich_blocking(album_key: &str, app: &tauri::AppHandle) -> Result<NeteaseReport, String> {
    let state = app.state::<AppState>();

    // 触碰音频文件 mtime,让增量扫描感知旁路元数据(cover.jpg/.lrc)已更新
    fn touch_audio(path: &Path) {
        let _ = std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .and_then(|f| f.set_modified(std::time::SystemTime::now()));
    }

    let tracks: Vec<model::Track> = {
        let lib = state.lib.lock().map_err(|_| "内部状态不可用".to_string())?;
        lib.tracks
            .iter()
            .filter(|t| format!("{}\u{1}{}", t.album_artist, t.album) == album_key)
            .cloned()
            .collect()
    };
    if tracks.is_empty() {
        return Err("未找到该专辑的曲目".into());
    }

    let client = netease_client();

    let mut report = NeteaseReport { cover: false, lyrics: 0, skipped: 0 };

    // 封面:存在无封面曲目时才匹配(先按专辑名搜专辑图,再回退按歌名搜歌曲所属专辑图)
    if tracks.iter().any(|t| t.cover.is_none()) {
        let first = &tracks[0];
        match netease_album_cover(&client, &first.album, &first.title) {
            Some(pic_url) => match client.get(&pic_url).send() {
                Ok(resp) if resp.status().is_success() => match resp.bytes() {
                    Ok(bytes) => {
                        let mut dirs: Vec<PathBuf> = tracks
                            .iter()
                            .filter(|t| t.cover.is_none())
                            .filter_map(|t| Path::new(&t.path).parent().map(|p| p.to_path_buf()))
                            .collect();
                        dirs.sort();
                        dirs.dedup();
                        for d in dirs {
                            let _ = fs::write(d.join("cover.jpg"), &bytes);
                        }
                        // 让该专辑所有无封面曲目在增量扫描中被重新解析
                        for t in tracks.iter().filter(|t| t.cover.is_none()) {
                            touch_audio(Path::new(&t.path));
                        }
                        report.cover = true;
                    }
                    Err(_) => report.skipped += 1,
                },
                _ => report.skipped += 1,
            },
            None => report.skipped += 1,
        }
    }

    // 歌词:逐首匹配无 .lrc 的曲目
    for t in tracks.iter().filter(|t| t.lrc_path.is_none()) {
        match netease_lyric(&client, &t.title, &t.artist) {
            Some(text) => {
                let parent = Path::new(&t.path).parent().unwrap_or_else(|| Path::new("."));
                let stem = Path::new(&t.path)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "track".into());
                let dest = parent.join(format!("{stem}.lrc"));
                if fs::write(&dest, text).is_ok() {
                    touch_audio(Path::new(&t.path));
                    report.lyrics += 1;
                } else {
                    report.skipped += 1;
                }
            }
            None => report.skipped += 1,
        }
    }
    Ok(report)
}

pub fn netease_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
        )
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("网络客户端初始化失败")
}

fn netease_search(client: &reqwest::blocking::Client, keyword: &str, search_type: u32, limit: u32) -> Option<serde_json::Value> {
    let resp = client
        .post("https://music.163.com/api/search/get/web")
        .header("Referer", "https://music.163.com")
        .header("Cookie", "os=pc; appver=2.9.7")
        .form(&[
            ("s", keyword),
            ("type", &search_type.to_string()),
            ("limit", &limit.to_string()),
        ])
        .send()
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json::<serde_json::Value>().ok()
}

/// 专辑封面:先按专辑名搜专辑,搜不到再按歌名搜歌曲取其所属专辑图
pub fn netease_album_cover(client: &reqwest::blocking::Client, album: &str, fallback_title: &str) -> Option<String> {
    if let Some(v) = netease_search(client, album, 10, 5) {
        if let Some(url) = v.pointer("/result/albums/0/picUrl").and_then(|x| x.as_str()) {
            return Some(url.replace("http://", "https://"));
        }
    }
    let v = netease_search(client, fallback_title, 1, 5)?;
    v.pointer("/result/songs/0/album/picUrl")
        .and_then(|x| x.as_str())
        .map(|s| s.replace("http://", "https://"))
}

/// 歌词:按歌名搜索,优先选艺人名能对上的结果
pub fn netease_lyric(client: &reqwest::blocking::Client, title: &str, artist: &str) -> Option<String> {
    let v = netease_search(client, title, 1, 5)?;
    let songs = v.pointer("/result/songs")?.as_array()?;
    let segments: Vec<&str> = artist.split(['/','&',',','、']).map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let pick = songs
        .iter()
        .find(|s| {
            s["artists"]
                .as_array()
                .map(|arr| {
                    arr.iter().any(|a| {
                        a["name"]
                            .as_str()
                            .map(|n| segments.iter().any(|seg| n.contains(seg) || seg.contains(n)))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        })
        .or_else(|| songs.first())?;
    let id = pick["id"].as_i64()?;
    let url = format!("https://music.163.com/api/song/lyric?id={id}&lv=1&kv=1&tv=-1");
    let resp = client
        .get(&url)
        .header("Referer", "https://music.163.com")
        .header("Cookie", "os=pc; appver=2.9.7")
        .send()
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v = resp.json::<serde_json::Value>().ok()?;
    let text = v.pointer("/lrc/lyric")?.as_str()?.trim().to_string();
    (!text.is_empty()).then_some(text)
}

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
            // 首次启动即建好导入目录并登记为扫描来源
            ensure_import_dir(&app.state::<AppState>()).ok();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_library,
            add_folder,
            remove_folder,
            pick_music_folder,
            scan_library,
            get_lyrics,
            get_import_dir,
            open_import_dir,
            import_paths,
            import_tmc,
            export_tmc,
            pick_tmc_file,
            pick_tmc_dest,
            netease_enrich_album
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
