mod lyrics;
mod model;
mod scanner;

use model::Library;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, State, WindowEvent,
};

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn toggle_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            show_main(app);
        }
    }
}

pub struct AppState {
    pub lib: Mutex<Library>,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    /// 扫描重入闸:同一时刻只允许一次全量扫描
    pub scan_gate: Mutex<()>,
}

pub(crate) const LIBRARY_FILE: &str = "library.json";
/// 应用自带的音乐导入目录(data_dir/Music),随应用首次启动创建并登记;
/// 扫描器会把顶层散装音频按「艺人\专辑」归位到该目录下
pub(crate) const IMPORT_DIR_NAME: &str = "Music";

/// 确保导入目录存在,并把它登记进曲库扫描来源。
/// 先落盘成功再改内存,落盘失败时内存态不发生变化。
fn ensure_import_dir(state: &AppState) -> Result<PathBuf, String> {
    let dir = state.data_dir.join(IMPORT_DIR_NAME);
    fs::create_dir_all(&dir).map_err(|e| format!("创建导入目录失败: {e}"))?;
    let mut lib = state.lib.lock().map_err(|_| "内部状态不可用".to_string())?;
    let path = dir.to_string_lossy().to_string();
    if !lib.folders.iter().any(|f| f == &path) {
        lib.folders.push(path);
        if let Err(e) = lib.save(&state.data_dir.join(LIBRARY_FILE)) {
            lib.folders.pop();
            return Err(e.to_string());
        }
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
    duplicates: u32,
}

/// 单文件复制结果:成功 / 内容重复跳过 / 不支持或已存在跳过
enum CopyResult {
    Copied,
    Duplicate,
    Skipped,
}

/// 拖拽导入:整个文件夹登记为扫描来源(不复制),散装音频/歌词文件复制进导入目录
#[tauri::command]
async fn import_paths(paths: Vec<String>, app: tauri::AppHandle) -> Result<ImportReport, String> {
    tauri::async_runtime::spawn_blocking(move || import_paths_blocking(&paths, &app))
        .await
        .map_err(|e| e.to_string())?
}

fn import_paths_blocking(paths: &[String], app: &tauri::AppHandle) -> Result<ImportReport, String> {
    let state = app.state::<AppState>();
    let import_dir = ensure_import_dir(&state)?;
    let mut report = ImportReport {
        folders_added: 0,
        files_copied: 0,
        skipped: 0,
        duplicates: 0,
    };
    let mut lib = state.lib.lock().map_err(|_| "内部状态不可用".to_string())?;
    // 内容去重基准:曲库已有文件的 (路径, 大小) + 本次会话刚复制的文件
    let mut known: Vec<(String, u64)> = lib
        .tracks
        .iter()
        .map(|t| (t.path.clone(), t.size))
        .collect();
    for raw in paths {
        let path = Path::new(raw);
        if path.is_dir() {
            let dir_str = path.to_string_lossy().to_string();
            if !lib.folders.iter().any(|f| f == &dir_str) {
                lib.folders.push(dir_str);
                report.folders_added += 1;
            }
        } else if path.is_file() {
            match copy_into_import_dir(path, &import_dir, &known) {
                Ok(CopyResult::Copied) => {
                    if let (Ok(m), Some(name)) = (fs::metadata(path), path.file_name()) {
                        known.push((import_dir.join(name).to_string_lossy().to_string(), m.len()));
                    }
                    report.files_copied += 1;
                }
                Ok(CopyResult::Duplicate) => report.duplicates += 1,
                _ => report.skipped += 1,
            }
        } else {
            report.skipped += 1;
        }
    }
    lib.save(&state.data_dir.join(LIBRARY_FILE))
        .map_err(|e| e.to_string())?;
    Ok(report)
}

/// 用路径组件判断 target 是否位于 dir 之下(与字符串前缀匹配不同,
/// 不会把 F:\music2 误当成 F:\music 的子目录)
fn is_under(child: &str, dir: &str) -> bool {
    Path::new(child).starts_with(Path::new(dir))
}

/// 两个文件内容是否完全一致(仅在大小相同时调用才有意义)
fn same_file_content(a: &Path, b: &Path) -> bool {
    use std::io::Read;
    let (Ok(mut fa), Ok(mut fb)) = (fs::File::open(a), fs::File::open(b)) else {
        return false;
    };
    let mut ba = [0u8; 64 * 1024];
    let mut bb = [0u8; 64 * 1024];
    loop {
        let (Ok(n1), Ok(n2)) = (fa.read(&mut ba), fb.read(&mut bb)) else {
            return false;
        };
        if n1 != n2 {
            return false;
        }
        if n1 == 0 {
            return true;
        }
        if ba[..n1] != bb[..n2] {
            return false;
        }
    }
}

/// 把单个文件复制进导入目录;只收音频与 .lrc 歌词。
/// 去重规则:同名同大小视为已存在;与曲库或本次会话已知文件大小相同且内容一致视为重复;
/// 同名但大小不同视为新版本,覆盖旧文件。
fn copy_into_import_dir(
    src: &Path,
    import_dir: &Path,
    known: &[(String, u64)],
) -> Result<CopyResult, String> {
    let supported = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let e = e.to_ascii_lowercase();
            scanner::EXTENSIONS.contains(&e.as_str()) || e == "lrc"
        })
        .unwrap_or(false);
    if !supported {
        return Ok(CopyResult::Skipped);
    }
    let Some(name) = src.file_name() else {
        return Ok(CopyResult::Skipped);
    };
    let dest = import_dir.join(name);
    if dest.exists() {
        let same = match (fs::metadata(src), fs::metadata(&dest)) {
            (Ok(a), Ok(b)) => a.len() == b.len(),
            _ => false,
        };
        if same {
            return Ok(CopyResult::Skipped);
        }
    } else if let Ok(m) = fs::metadata(src) {
        // 内容级去重:与已知文件大小相同再比字节(候选最多比 5 个,避免大文件 IO 过久)
        let size = m.len();
        let mut checked = 0u32;
        for (p, s) in known {
            if *s == size && checked < 5 {
                checked += 1;
                if same_file_content(src, Path::new(p)) {
                    return Ok(CopyResult::Duplicate);
                }
            }
        }
    }
    fs::copy(src, &dest)
        .map(|_| CopyResult::Copied)
        .map_err(|e| format!("复制 {} 失败: {e}", src.display()))
}

// ===== TMC 音乐包(.tmc = 标准 7z:音频 + 同名 .lrc 歌词 + cover 封面 + meta.json) =====

pub(crate) fn sanitize_name(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => out.push('_'),
            c if (c as u32) < 32 => {}
            c => out.push(c),
        }
    }
    // Windows 文件名:首尾空白与结尾的点会被静默吞掉或报错,保留名不可用
    let mut t = out.trim().trim_end_matches(['.', ' ']).to_string();
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED.contains(&t.to_ascii_uppercase().as_str()) {
        t.insert(0, '_');
    }
    if t.is_empty() {
        "track".into()
    } else {
        t
    }
}

/// 解包 .tmc 到导入目录下的同名子目录,由扫描器拾取(meta.json 会补齐缺失标签)
#[tauri::command]
async fn import_tmc(path: String, app: tauri::AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        import_tmc_file(Path::new(&path), &state)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn import_tmc_file(src: &Path, state: &AppState) -> Result<String, String> {
    if !src.is_file() {
        return Err("文件不存在".into());
    }
    let import_dir = ensure_import_dir(state)?;
    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "tmc".into());
    let dest = import_dir.join(&stem);
    if dest.exists() {
        fs::remove_dir_all(&dest).map_err(|e| format!("清理旧目录失败: {e}"))?;
    }
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    sevenz_rust::decompress_file(src, &dest).map_err(|e| format!("解包失败: {e}"))?;
    Ok(dest.to_string_lossy().to_string())
}

/// 处理"打开方式"/命令行传入的文件:.tmc 解包导入,音频复制进导入目录,完成后重扫
fn handle_open_paths(app: &tauri::AppHandle, args: &[String]) {
    let paths: Vec<PathBuf> = args
        .iter()
        .skip(1)
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .collect();
    if paths.is_empty() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // 7z 解压 + 全量扫描都是重活,放阻塞线程池,避免卡 tokio worker
        let worker = tauri::async_runtime::spawn_blocking(move || {
            let state = app.state::<AppState>();
            let import_dir = match ensure_import_dir(&state) {
                Ok(d) => d,
                Err(e) => {
                    let _ = app.emit("import-error", format!("导入目录不可用: {e}"));
                    return;
                }
            };
            let known: Vec<(String, u64)> = state
                .lib
                .lock()
                .map(|lib| {
                    lib.tracks
                        .iter()
                        .map(|t| (t.path.clone(), t.size))
                        .collect()
                })
                .unwrap_or_default();
            let mut failures: Vec<String> = Vec::new();
            for p in &paths {
                let ext = p
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase().to_string())
                    .unwrap_or_default();
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| p.to_string_lossy().to_string());
                let result = if ext == "tmc" {
                    import_tmc_file(p, &state).map(|_| ())
                } else if scanner::EXTENSIONS.contains(&ext.as_str()) {
                    copy_into_import_dir(p, &import_dir, &known).map(|_| ())
                } else {
                    failures.push(format!("{name}: 不支持的格式"));
                    continue;
                };
                if let Err(e) = result {
                    failures.push(format!("{name}: {e}"));
                }
            }
            if let Err(e) = scanner::run_scan(&app) {
                failures.push(format!("扫描失败: {e}"));
            }
            if !failures.is_empty() {
                // 汇总失败清单推给前端(状态条最多展示有限字数,取前 3 条)
                let summary = failures
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ");
                let more = if failures.len() > 3 {
                    format!(" 等 {} 项", failures.len())
                } else {
                    String::new()
                };
                let _ = app.emit("import-error", format!("{summary}{more}"));
            }
            let _ = app.emit("library-changed", ());
        });
        let _ = worker.await;
    });
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
    fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let mut dest_path = PathBuf::from(dest);
    // 文件名清洗:标签可能含 \ / 等文件系统非法字符
    if let Some(parent) = dest_path.parent() {
        let stem = dest_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        dest_path = parent.join(sanitize_name(&stem));
    }
    if dest_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase() != "tmc")
        .unwrap_or(true)
    {
        dest_path.set_extension("tmc");
    }
    // 批量导出时同名(同标题)曲目自动加序号,避免相互覆盖
    if dest_path.exists() {
        let parent = dest_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let stem = dest_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "track".into());
        let mut i = 2;
        loop {
            let candidate = parent.join(format!("{stem} ({i}).tmc"));
            if !candidate.exists() {
                dest_path = candidate;
                break;
            }
            i += 1;
        }
    }
    if let Some(parent) = dest_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut writer = SevenZWriter::create(&dest_path).map_err(|e| format!("创建 TMC 失败: {e}"))?;
    // 打开失败必须报错:SevenZWriter 收到 None reader 会静默写入 size=0 的空条目,
    // 产出损坏的 .tmc(典型场景:音频正被播放器占用)
    let audio_file =
        fs::File::open(audio).map_err(|e| format!("打开音频失败 {}: {e}", audio.display()))?;
    writer
        .push_archive_entry(
            SevenZArchiveEntry::from_path(audio, format!("{stem}.{ext}")),
            Some(audio_file),
        )
        .map_err(|e| e.to_string())?;
    if let Some(lrc) = &t.lrc_path {
        let lrc_path = Path::new(lrc);
        if lrc_path.is_file() {
            let lrc_file = fs::File::open(lrc_path)
                .map_err(|e| format!("打开歌词失败 {}: {e}", lrc_path.display()))?;
            writer
                .push_archive_entry(
                    SevenZArchiveEntry::from_path(lrc_path, format!("{stem}.lrc")),
                    Some(lrc_file),
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
            let cname = if cext == "png" {
                "cover.png"
            } else {
                "cover.jpg"
            };
            let cover_file = fs::File::open(cover_path)
                .map_err(|e| format!("打开封面失败 {}: {e}", cover_path.display()))?;
            writer
                .push_archive_entry(
                    SevenZArchiveEntry::from_path(cover_path, cname.to_string()),
                    Some(cover_file),
                )
                .map_err(|e| e.to_string())?;
        }
    }
    let meta_file = fs::File::open(&meta_path).map_err(|e| format!("打开临时元数据失败: {e}"))?;
    writer
        .push_archive_entry(
            SevenZArchiveEntry::from_path(&meta_path, "meta.json".to_string()),
            Some(meta_file),
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
    // 标签里的标题可能含 \ / 等文件系统非法字符,默认名先清洗
    let name = sanitize_name(&default_name);
    rfd::AsyncFileDialog::new()
        .set_title("导出 TMC 音乐包")
        .add_filter("TMC 音乐包 (*.tmc)", &["tmc"])
        .set_file_name(format!("{name}.tmc"))
        .save_file()
        .await
        .map(|f| f.path().to_string_lossy().to_string())
}

#[tauri::command]
async fn pick_audio_files() -> Option<Vec<String>> {
    rfd::AsyncFileDialog::new()
        .set_title("选择音乐文件")
        .add_filter(
            "音频文件",
            &["mp3", "m4a", "flac", "ogg", "oga", "opus", "wav"],
        )
        .pick_files()
        .await
        .map(|files| {
            files
                .iter()
                .map(|f| f.path().to_string_lossy().to_string())
                .collect()
        })
}

/// 批量导出的目标文件夹
#[tauri::command]
async fn pick_export_dir() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("选择导出位置")
        .pick_folder()
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

fn netease_enrich_blocking(
    album_key: &str,
    app: &tauri::AppHandle,
) -> Result<NeteaseReport, String> {
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

    let mut report = NeteaseReport {
        cover: false,
        lyrics: 0,
        skipped: 0,
    };

    // 封面:存在无封面曲目时才匹配(先按专辑名搜专辑图,再回退按歌名搜歌曲所属专辑图)
    if tracks.iter().any(|t| t.cover.is_none()) {
        let first = &tracks[0];
        match netease_album_cover(&client, &first.album, &first.album_artist, &first.title) {
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
                        // 混居目录(同一目录里还有其他专辑的曲目)不能写共享 cover.jpg,
                        // 否则目录里所有专辑都会套上同一张封面
                        let multi_dirs: std::collections::HashSet<PathBuf> = {
                            let lib = state.lib.lock().map_err(|_| "内部状态不可用".to_string())?;
                            dirs.iter()
                                .filter(|d| {
                                    let mut seen = std::collections::HashSet::new();
                                    for t in &lib.tracks {
                                        if Path::new(&t.path)
                                            .parent()
                                            .map(|p| p == **d)
                                            .unwrap_or(false)
                                        {
                                            seen.insert(format!(
                                                "{}\u{1}{}",
                                                t.album_artist, t.album
                                            ));
                                        }
                                    }
                                    seen.len() > 1
                                })
                                .cloned()
                                .collect()
                        };
                        for d in &dirs {
                            if multi_dirs.contains(d) {
                                // 写曲目专属旁路封面:{歌名}.cover.jpg
                                for t in tracks.iter().filter(|t| t.cover.is_none()) {
                                    let tp = Path::new(&t.path);
                                    if tp.parent().map(|p| p == d.as_path()).unwrap_or(false) {
                                        let stem = tp
                                            .file_stem()
                                            .map(|s| s.to_string_lossy().to_string())
                                            .unwrap_or_default();
                                        let _ =
                                            fs::write(d.join(format!("{stem}.cover.jpg")), &bytes);
                                    }
                                }
                            } else {
                                let _ = fs::write(d.join("cover.jpg"), &bytes);
                            }
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

    // 歌词:批量并行匹配无 .lrc 的曲目(每首 2 次 HTTP 往返,串行在大专辑下太慢)
    let targets: Vec<&model::Track> = tracks.iter().filter(|t| t.lrc_path.is_none()).collect();
    let results: Vec<Option<String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = targets
            .chunks(4)
            .map(|chunk| {
                let client = &client;
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|t| netease_lyric(client, &t.title, &t.artist))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });
    for (t, text) in targets.iter().zip(results) {
        match text {
            Some(text) => {
                let parent = Path::new(&t.path)
                    .parent()
                    .unwrap_or_else(|| Path::new("."));
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
    static CLIENT: std::sync::OnceLock<reqwest::blocking::Client> = std::sync::OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::blocking::Client::builder()
                .user_agent(
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
                )
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::new())
        })
        .clone()
}

/// 归一化标签用于跨源比对:繁体转简体 + 小写 + 去空白(本地标签常见繁体/大小写差异)
fn normalize_tag(s: &str) -> String {
    use character_converter::CharacterConverter;
    static CONVERTER: std::sync::OnceLock<CharacterConverter> = std::sync::OnceLock::new();
    let converter = CONVERTER.get_or_init(CharacterConverter::new);
    converter
        .traditional_to_simplified(s)
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// 归一化后的双向包含比对
fn name_matches(a: &str, b: &str) -> bool {
    let (a, b) = (normalize_tag(a), normalize_tag(b));
    !a.is_empty() && !b.is_empty() && (a.contains(&b) || b.contains(&a))
}

/// 艺人组任一分段与候选名比对成功即通过
fn artist_matches(artist: &str, candidate: &str) -> bool {
    artist
        .split(['/', '&', ',', '、'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .any(|seg| name_matches(seg, candidate))
}

fn netease_search(
    client: &reqwest::blocking::Client,
    keyword: &str,
    search_type: u32,
    limit: u32,
) -> Option<serde_json::Value> {
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

/// 专辑封面:按专辑名搜专辑并校验艺人,无命中再按歌名搜歌曲取其所属专辑图。
/// 不校验艺人会拿到同名但毫无关系的专辑封面(如「Control」会命中 Janet Jackson)。
pub fn netease_album_cover(
    client: &reqwest::blocking::Client,
    album: &str,
    album_artist: &str,
    fallback_title: &str,
) -> Option<String> {
    if let Some(v) = netease_search(client, album, 10, 5) {
        if let Some(albums) = v.pointer("/result/albums").and_then(|x| x.as_array()) {
            // 优先取艺人对得上的专辑,全对不上时不用专辑搜索结果(大概率是同名的别家)
            if let Some(pic) = albums
                .iter()
                .find(|a| {
                    a["artist"]["name"]
                        .as_str()
                        .map(|n| artist_matches(album_artist, n))
                        .unwrap_or(false)
                })
                .and_then(|a| a["picUrl"].as_str())
            {
                return Some(pic.replace("http://", "https://"));
            }
        }
    }
    // 回退:按歌名搜歌曲,优先艺人对得上的,否则取首个
    let v = netease_search(client, fallback_title, 1, 5)?;
    let songs = v.pointer("/result/songs")?.as_array()?;
    let pic = songs
        .iter()
        .find(|s| {
            s["artists"]
                .as_array()
                .map(|arr| {
                    arr.iter().any(|a| {
                        a["name"]
                            .as_str()
                            .map(|n| artist_matches(album_artist, n))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        })
        .or_else(|| songs.first())?["album"]["picUrl"]
        .as_str()
        .map(|s| s.replace("http://", "https://"))?;
    Some(pic)
}

/// 歌词:按歌名搜索,优先选艺人名能对上的结果(繁简/大小写归一化比对)
pub fn netease_lyric(
    client: &reqwest::blocking::Client,
    title: &str,
    artist: &str,
) -> Option<String> {
    let v = netease_search(client, title, 1, 5)?;
    let songs = v.pointer("/result/songs")?.as_array()?;
    let pick = songs
        .iter()
        .find(|s| {
            s["artists"]
                .as_array()
                .map(|arr| {
                    arr.iter().any(|a| {
                        a["name"]
                            .as_str()
                            .map(|n| artist_matches(artist, n))
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

// ===== 格式关联(HKCU 注册到"打开方式",无需管理员) =====

/// 删除曲目:音频与同名歌词移入回收站,曲库同步移除;
/// 导入目录内的子目录若已无音频文件则一并清理
#[tauri::command]
async fn delete_tracks(ids: Vec<String>, app: tauri::AppHandle) -> Result<DeleteReport, String> {
    tauri::async_runtime::spawn_blocking(move || delete_tracks_blocking(&ids, &app))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteReport {
    pub deleted: u32,
    pub failed: u32,
}

fn delete_tracks_blocking(ids: &[String], app: &tauri::AppHandle) -> Result<DeleteReport, String> {
    let state = app.state::<AppState>();
    let import_dir = state.data_dir.join("Music");

    // 1) 锁内:取出匹配曲目并先从曲库移除(避免长时间持锁做文件删除)
    let to_delete: Vec<model::Track> = {
        let mut lib = state.lib.lock().map_err(|_| "内部状态不可用".to_string())?;
        let (del, kept): (Vec<model::Track>, Vec<model::Track>) =
            lib.tracks.drain(..).partition(|t| ids.contains(&t.id));
        lib.tracks = kept;
        lib.save(&state.data_dir.join(LIBRARY_FILE))
            .map_err(|e| e.to_string())?;
        del
    };

    // 2) 锁外:移入回收站(音频 + 同名歌词),回收站可恢复
    let mut deleted = 0u32;
    let mut parents: Vec<PathBuf> = Vec::new();
    let mut failed_tracks: Vec<model::Track> = Vec::new();
    for t in &to_delete {
        let p = Path::new(&t.path);
        match trash::delete(p) {
            Ok(()) => {
                deleted += 1;
                if let Some(parent) = p.parent() {
                    if let Some(stem) = p.file_stem() {
                        let _ =
                            trash::delete(parent.join(format!("{}.lrc", stem.to_string_lossy())));
                    }
                    let parent = parent.to_path_buf();
                    if !parents.contains(&parent) {
                        parents.push(parent);
                    }
                }
            }
            // 回收站删除失败(文件被占用/权限不足)→ 稍后回填曲库,避免幽灵状态
            Err(_) => failed_tracks.push(t.clone()),
        }
    }

    // 3) 失败的回填曲库并落盘
    if !failed_tracks.is_empty() {
        let mut lib = state.lib.lock().map_err(|_| "内部状态不可用".to_string())?;
        lib.tracks.extend(failed_tracks.iter().cloned());
        lib.save(&state.data_dir.join(LIBRARY_FILE))
            .map_err(|e| e.to_string())?;
        eprintln!(
            "[delete] {} 首删除失败(文件被占用?),已保留曲库记录",
            failed_tracks.len()
        );
    }

    // 4) 导入目录的子目录(非根)如果没有音频文件了,连同残留的 meta/封面一起清掉
    for d in &parents {
        if d.starts_with(&import_dir) && *d != import_dir {
            let has_audio = std::fs::read_dir(d)
                .map(|rd| {
                    rd.filter_map(|e| e.ok()).any(|e| {
                        e.path()
                            .extension()
                            .map(|x| {
                                let x = x.to_string_lossy().to_lowercase();
                                scanner::EXTENSIONS.contains(&x.as_str())
                            })
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);
            if !has_audio {
                let _ = fs::remove_dir_all(d);
            }
        }
    }

    let failed = failed_tracks.len() as u32;
    Ok(DeleteReport { deleted, failed })
}

pub const ASSOC_EXTS: &[(&str, &str)] = &[
    ("tmc", "TMC 音乐包"),
    ("mp3", "MP3 音频"),
    ("flac", "FLAC 音频"),
    ("m4a", "M4A 音频"),
    ("wav", "WAV 音频"),
    ("ogg", "OGG 音频"),
    ("opus", "OPUS 音频"),
];

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssocState {
    ext: String,
    label: String,
    registered: bool,
}

fn assoc_desc(ext: &str) -> &'static str {
    ASSOC_EXTS
        .iter()
        .find(|(e, _)| *e == ext)
        .map(|(_, d)| *d)
        .unwrap_or("音频")
}

#[tauri::command]
fn get_associations() -> Vec<AssocState> {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        ASSOC_EXTS
            .iter()
            .map(|(ext, label)| {
                let registered = hkcu
                    .open_subkey(format!(r"Software\Classes\.{ext}\OpenWithProgids"))
                    .ok()
                    .map(|k| {
                        k.get_value::<String, _>(format!("TauriMusic.{ext}"))
                            .is_ok()
                    })
                    .unwrap_or(false);
                AssocState {
                    ext: ext.to_string(),
                    label: label.to_string(),
                    registered,
                }
            })
            .collect()
    }
    #[cfg(not(windows))]
    {
        ASSOC_EXTS
            .iter()
            .map(|(ext, label)| AssocState {
                ext: ext.to_string(),
                label: label.to_string(),
                registered: false,
            })
            .collect()
    }
}

#[tauri::command]
fn set_association(ext: String, enable: bool) -> Result<(), String> {
    let desc = assoc_desc(&ext);
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;

    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let progid = format!("TauriMusic.{ext}");
        if enable {
            let cmd_key = hkcu
                .create_subkey(format!(r"Software\Classes\{progid}\shell\open\command"))
                .map_err(|e| e.to_string())?
                .0;
            cmd_key
                .set_value("", &format!("\"{}\" \"%1\"", exe.display()))
                .map_err(|e| e.to_string())?;
            let icon_key = hkcu
                .create_subkey(format!(r"Software\Classes\{progid}\DefaultIcon"))
                .map_err(|e| e.to_string())?
                .0;
            icon_key
                .set_value("", &format!("{},0", exe.display()))
                .map_err(|e| e.to_string())?;
            let id_key = hkcu
                .create_subkey(format!(r"Software\Classes\{progid}"))
                .map_err(|e| e.to_string())?
                .0;
            id_key
                .set_value("", &format!("TauriMusic {desc}"))
                .map_err(|e| e.to_string())?;
            let ext_key = hkcu
                .create_subkey(format!(r"Software\Classes\.{ext}\OpenWithProgids"))
                .map_err(|e| e.to_string())?
                .0;
            ext_key
                .set_value(&progid, &String::new())
                .map_err(|e| e.to_string())?;
            // 用户级默认 ProgID:无 UserChoice 时双击即用 TauriMusic(优先于 HKLM)
            let def_key = hkcu
                .create_subkey(format!(r"Software\Classes\.{ext}"))
                .map_err(|e| e.to_string())?
                .0;
            def_key.set_value("", &progid).map_err(|e| e.to_string())?;
        } else {
            if let Ok(k) = hkcu.open_subkey_with_flags(
                format!(r"Software\Classes\.{ext}\OpenWithProgids"),
                KEY_WRITE,
            ) {
                let _ = k.delete_value(&progid);
            }
            // 仅当用户级默认仍指向我们时才摘除,不破坏其他应用的关联
            if let Ok(k) =
                hkcu.open_subkey_with_flags(format!(r"Software\Classes\.{ext}"), KEY_WRITE)
            {
                let current: Result<String, _> = k.get_value("");
                if matches!(current, Ok(v) if v == progid) {
                    let _ = k.delete_value("");
                }
            }
            let _ = hkcu.delete_subkey_all(format!(r"Software\Classes\{progid}"));
        }
        // 通知 Explorer 立即刷新关联与图标缓存
        #[link(name = "shell32")]
        extern "system" {
            fn SHChangeNotify(
                wEventId: u32,
                uFlags: u32,
                dwItem1: *const std::ffi::c_void,
                dwItem2: *const std::ffi::c_void,
            );
        }
        unsafe {
            SHChangeNotify(0x0800_0000, 0x0000, std::ptr::null(), std::ptr::null());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (desc, exe, enable);
        Err("仅支持 Windows".into())
    }
}

#[tauri::command]
fn get_library(state: State<AppState>) -> serde_json::Value {
    // 锁内完成序列化:大曲库时避免整库深拷贝再二次序列化
    match state.lib.lock() {
        Ok(lib) => serde_json::to_value(&*lib).unwrap_or_default(),
        Err(e) => serde_json::to_value(&*e.into_inner()).unwrap_or_default(),
    }
}

#[tauri::command]
fn add_folder(path: String, state: State<AppState>) -> Result<(), String> {
    if !Path::new(&path).is_dir() {
        return Err("该路径不是一个文件夹".into());
    }
    let mut lib = state.lib.lock().map_err(|_| "内部状态不可用".to_string())?;
    if !lib.folders.iter().any(|f| f == &path) {
        lib.folders.push(path);
        if let Err(e) = lib.save(&state.data_dir.join(LIBRARY_FILE)) {
            lib.folders.pop();
            return Err(e.to_string());
        }
    }
    Ok(())
}

#[tauri::command]
fn remove_folder(path: String, state: State<AppState>) -> Result<(), String> {
    let mut lib = state.lib.lock().map_err(|_| "内部状态不可用".to_string())?;
    lib.folders.retain(|f| f != &path);
    // 组件级前缀匹配:字符串 starts_with 会把 F:\music2 误当成 F:\music 的子目录
    lib.tracks.retain(|t| !is_under(&t.path, &path));
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
async fn get_lyrics(id: String, app: tauri::AppHandle) -> Option<lyrics::LyricsPayload> {
    // 锁内只查表取 Track;歌词解析(lofty 音频探测)移到阻塞线程
    let track = {
        let state = app.state::<AppState>();
        let lib = state.lib.lock().ok()?;
        lib.tracks.iter().find(|t| t.id == id).cloned()
    }?;
    tauri::async_runtime::spawn_blocking(move || lyrics::load_for_track(&track))
        .await
        .ok()?
}

// ===== 资源占用(关于页展示) =====

static RES_SYS: std::sync::OnceLock<Mutex<sysinfo::System>> = std::sync::OnceLock::new();

/// 主进程资源占用:内存 MB + CPU 百分比(CPU 为两次调用间的均值,首次为 0)
#[derive(serde::Serialize)]
struct ResourceUsage {
    memory_mb: f64,
    cpu: f32,
}

#[tauri::command]
fn get_resource_usage() -> ResourceUsage {
    let sys_mutex = RES_SYS.get_or_init(|| Mutex::new(sysinfo::System::new()));
    let mut sys = sys_mutex.lock().unwrap_or_else(|e| e.into_inner());
    let (memory_mb, cpu) = match sysinfo::get_current_pid() {
        Ok(pid) => {
            sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
            sys.process(pid)
                .map(|p| (p.memory() as f64 / 1048576.0, p.cpu_usage()))
                .unwrap_or((0.0, 0.0))
        }
        Err(_) => (0.0, 0.0),
    };
    ResourceUsage { memory_mb, cpu }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 二次启动时唤起已有窗口,并把传入的文件交给运行中的实例导入
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            show_main(app);
            handle_open_paths(app, &args);
        }))
        // 点 × 隐藏到托盘常驻,托盘菜单"退出"才真正退出
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let handle = app.handle();
            let data_dir = handle.path().app_data_dir().expect("无法确定应用数据目录");
            let cache_dir = handle.path().app_cache_dir().expect("无法确定应用缓存目录");
            std::fs::create_dir_all(&data_dir).ok();
            std::fs::create_dir_all(cache_dir.join("covers")).ok();
            let lib = Library::load(&data_dir.join(LIBRARY_FILE));
            app.manage(AppState {
                lib: Mutex::new(lib),
                data_dir,
                cache_dir,
                scan_gate: Mutex::new(()),
            });
            // 首次启动即建好导入目录并登记为扫描来源
            ensure_import_dir(&app.state::<AppState>()).ok();
            // 命令行/双击"打开方式"传入的文件
            let cli_args: Vec<String> = std::env::args().collect();
            handle_open_paths(handle, &cli_args);

            // 托盘常驻:左键切换显示/隐藏,右键菜单可显示或退出
            let show = MenuItem::with_id(app, "show", "显示 TauriMusic", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().expect("缺少应用图标").clone())
                .tooltip("TauriMusic")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_main(tray.app_handle());
                    }
                })
                .build(app)?;
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
            pick_audio_files,
            pick_export_dir,
            netease_enrich_album,
            get_associations,
            set_association,
            delete_tracks,
            get_resource_usage
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{
        artist_matches, copy_into_import_dir, is_under, name_matches, same_file_content,
        sanitize_name, CopyResult,
    };
    use std::fs;

    #[test]
    fn is_under_uses_path_components() {
        // 字符串前缀匹配会把 F:\music2 误当成 F:\music 的子目录,组件匹配不会
        assert!(is_under(r"F:\music\a.mp3", r"F:\music"));
        assert!(is_under(r"F:\music\sub\b.mp3", r"F:\music"));
        assert!(is_under(r"F:\music", r"F:\music"));
        assert!(!is_under(r"F:\music2\a.mp3", r"F:\music"));
        assert!(!is_under(r"F:\musica\c.mp3", r"F:\music"));
    }

    #[test]
    fn content_compare_matches_identical_files() {
        let dir = std::env::temp_dir().join(format!("tm-dedup-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.bin");
        let b = dir.join("b.bin");
        let c = dir.join("c.bin");
        fs::write(&a, [1u8, 2, 3, 4]).unwrap();
        fs::write(&b, [1u8, 2, 3, 4]).unwrap();
        fs::write(&c, [1u8, 2, 3, 5]).unwrap();
        assert!(same_file_content(&a, &b));
        assert!(!same_file_content(&a, &c));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn import_rejects_duplicate_content() {
        let dir = std::env::temp_dir().join(format!("tm-import-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let imp = dir.join("imp");
        fs::create_dir_all(&imp).unwrap();

        // 曲库里已有的原始文件
        let origin = dir.join("song.mp3");
        fs::write(&origin, b"same-audio-content").unwrap();
        let known = vec![(origin.to_string_lossy().to_string(), 18)];

        // 同内容不同名 → 重复拒收
        let dup = dir.join("song (1).mp3");
        fs::write(&dup, b"same-audio-content").unwrap();
        assert!(matches!(
            copy_into_import_dir(&dup, &imp, &known),
            Ok(CopyResult::Duplicate)
        ));

        // 不同内容 → 正常复制
        let fresh = dir.join("other.mp3");
        fs::write(&fresh, b"different-content").unwrap();
        assert!(matches!(
            copy_into_import_dir(&fresh, &imp, &known),
            Ok(CopyResult::Copied)
        ));

        // 同名同大小 → 已存在跳过
        let origin_copy = dir.join("origin_copy");
        fs::copy(&origin, &origin_copy).unwrap();
        assert!(matches!(
            copy_into_import_dir(&origin_copy, &imp, &known),
            Ok(CopyResult::Skipped)
        ));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn name_matching_is_normalized() {
        // 大小写
        assert!(name_matches("Beyond", "BEYOND"));
        // 繁简
        assert!(name_matches("湯幻月", "汤幻月"));
        // 空串不匹配
        assert!(!name_matches("", "Beyond"));
        // 无关艺人
        assert!(!name_matches("Janet Jackson", "Beyond"));
    }

    #[test]
    fn artist_group_matches_any_segment() {
        // 多艺人组里任一命中即可,且繁体「湯」与简体「汤」互通
        assert!(artist_matches("湯幻月/小义学长", "小义学长"));
        assert!(artist_matches("湯幻月/小义学长", "汤幻月"));
        assert!(!artist_matches("湯幻月/小义学长", "张治环"));
    }

    #[test]
    fn sanitize_replaces_invalid_filename_chars() {
        assert_eq!(
            sanitize_name("a/b\\c:d*e?f\"g<h>i|j"),
            "a_b_c_d_e_f_g_h_i_j"
        );
        assert_eq!(sanitize_name("海阔天空"), "海阔天空");
        assert_eq!(sanitize_name("  Beyond  "), "Beyond");
        // 结尾的点与空格被 Windows 静默吞掉,一并去掉
        assert_eq!(sanitize_name("海阔天空. "), "海阔天空");
        // Windows 保留名加前缀
        assert_eq!(sanitize_name("CON"), "_CON");
        assert_eq!(sanitize_name("com1"), "_com1");
        // 控制字符剔除,空结果回退默认名
        assert_eq!(sanitize_name("\u{0}\u{1}"), "track");
        assert_eq!(sanitize_name(""), "track");
    }
}
