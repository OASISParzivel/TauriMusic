use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

use crate::model::{ScanReport, Track};
use crate::AppState;

pub const EXTENSIONS: &[&str] = &["mp3", "m4a", "flac", "ogg", "oga", "opus", "wav"];
/// 目录内兜底封面文件名(不含扩展名)
const COVER_NAMES: &[&str] = &["cover", "folder", "front", "album", "albumart"];

/// 全量扫描所有已登记的音乐文件夹,与旧曲库做增量合并后落盘。
pub fn run_scan(app: &AppHandle) -> Result<ScanReport, String> {
    let state = app.state::<AppState>();
    let (folders, existing) = {
        let lib = state.lib.lock().map_err(|_| "曲库状态不可用".to_string())?;
        (lib.folders.clone(), lib.tracks.clone())
    };

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64();
    let by_path: HashMap<String, Track> =
        existing.iter().map(|t| (t.path.clone(), t.clone())).collect();

    // 先收集文件清单(快),再逐个解析并上报进度
    let mut files: Vec<PathBuf> = Vec::new();
    for folder in &folders {
        for entry in walkdir::WalkDir::new(folder)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| e.file_name().to_string_lossy().chars().next() != Some('.'))
        {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() {
                continue;
            }
            let ext = entry
                .path()
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase());
            if matches!(ext.as_deref(), Some(e) if EXTENSIONS.contains(&e)) {
                files.push(entry.into_path());
            }
        }
    }
    let total = files.len();

    let covers_dir = state.cache_dir.join("covers");
    let _ = fs::create_dir_all(&covers_dir);

    // 专辑封面去重:同一张专辑只提取/落盘一次
    let mut album_covers: HashMap<String, String> = HashMap::new();
    let mut result: Vec<Track> = Vec::with_capacity(total);
    let mut seen: HashSet<String> = HashSet::new();
    let (mut added, mut updated, mut errors) = (0usize, 0usize, 0usize);

    for (i, path) in files.iter().enumerate() {
        let path_str = path.to_string_lossy().to_string();
        seen.insert(path_str.clone());

        let (mtime, size) = match fs::metadata(path) {
            Ok(m) => (
                m.modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0),
                m.len(),
            ),
            Err(_) => {
                errors += 1;
                continue;
            }
        };

        let cached = by_path.get(&path_str).cloned();
        if let Some(prev) = &cached {
            // 文件未变化:直接复用旧记录,不重复解析
            if (prev.mtime - mtime).abs() < 0.5 && prev.size == size {
                result.push(prev.clone());
                continue;
            }
        }

        match parse_track(path, mtime, size, now, &covers_dir, &mut album_covers) {
            Ok(mut track) => {
                if let Some(prev) = cached {
                    track.added_at = prev.added_at;
                    updated += 1;
                } else {
                    added += 1;
                }
                result.push(track);
            }
            Err(err) => {
                errors += 1;
                eprintln!("[scan] 解析失败 {path_str}: {err}");
                if let Some(prev) = cached {
                    result.push(prev);
                }
            }
        }

        if i % 20 == 0 || i + 1 == total {
            let _ = app.emit("scan-progress", json!({ "current": i + 1, "total": total }));
        }
    }

    let removed = existing.iter().filter(|t| !seen.contains(&t.path)).count();
    let report = ScanReport { added, updated, removed, total, errors };

    {
        let mut lib = state.lib.lock().map_err(|_| "曲库状态不可用".to_string())?;
        lib.tracks = result;
        if let Err(e) = lib.save(&state.data_dir.join("library.json")) {
            eprintln!("[scan] 曲库落盘失败: {e}");
        }
    }
    Ok(report)
}

fn parse_track(
    path: &Path,
    mtime: f64,
    size: u64,
    added_at: f64,
    covers_dir: &Path,
    album_covers: &mut HashMap<String, String>,
) -> Result<Track, String> {
    use lofty::file::{AudioFile, TaggedFileExt};
    use lofty::picture::MimeType;
    use lofty::probe::Probe;
    use lofty::tag::ItemKey;

    let tagged = Probe::open(path).map_err(|e| e.to_string())?.read().map_err(|e| e.to_string())?;
    let duration = tagged.properties().duration().as_secs_f64();
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let get = |key: &ItemKey| -> Option<String> {
        tag.and_then(|t| t.get_string(key))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let path_str = path.to_string_lossy().to_string();
    let parent = path.parent().unwrap_or_else(|| Path::new(""));

    // 标签缺失时回退到文件名,支持 "艺人 - 标题" 命名
    let mut title = get(&ItemKey::TrackTitle);
    let mut artist = get(&ItemKey::TrackArtist);
    if title.is_none() && artist.is_none() {
        if let Some((a, t)) = stem.split_once(" - ") {
            artist = Some(a.trim().to_string());
            title = Some(t.trim().to_string());
        }
    }
    let mut album = get(&ItemKey::AlbumTitle);
    let mut album_artist = get(&ItemKey::AlbumArtist);
    let mut year = get(&ItemKey::Year)
        .or_else(|| get(&ItemKey::RecordingDate))
        .and_then(|y| {
            y.chars()
                .take(4)
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<i32>()
                .ok()
        })
        .filter(|&y| (1000..=3000).contains(&y));
    let mut track_no = get(&ItemKey::TrackNumber).and_then(parse_slashed_number);
    let disc_no = get(&ItemKey::DiscNumber).and_then(parse_slashed_number);
    let mut genre = get(&ItemKey::Genre);

    // TMC 音乐包元数据:同级 meta.json 只补齐标签缺失的字段,不覆盖内嵌标签
    let meta_path = parent.join("meta.json");
    if meta_path.is_file() {
        if let Ok(v) =
            serde_json::from_str::<serde_json::Value>(&fs::read_to_string(&meta_path).unwrap_or_default())
        {
            let mget = |k: &str| -> Option<String> {
                v.get(k)
                    .and_then(|x| x.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            };
            title = title.or_else(|| mget("title"));
            artist = artist.or_else(|| mget("artist"));
            album = album.or_else(|| mget("album"));
            album_artist = album_artist.or_else(|| mget("albumArtist"));
            if year.is_none() {
                year = mget("year")
                    .and_then(|y| {
                        y.chars()
                            .take(4)
                            .filter(|c| c.is_ascii_digit())
                            .collect::<String>()
                            .parse::<i32>()
                            .ok()
                    })
                    .filter(|&y| (1000..=3000).contains(&y));
            }
            if track_no.is_none() {
                track_no = mget("trackNo").and_then(parse_slashed_number);
            }
            if genre.is_none() {
                genre = mget("genre");
            }
        }
    }

    let title = title.unwrap_or_else(|| stem.clone());
    let artist = artist.unwrap_or_else(|| "未知艺人".into());
    let album = album.unwrap_or_else(|| "未知专辑".into());
    let album_artist = album_artist.unwrap_or_else(|| artist.clone());

    // 歌词:同名 .lrc 伴生文件,或内嵌歌词标签
    let lrc = parent.join(format!("{stem}.lrc"));
    let lrc_path = lrc
        .is_file()
        .then(|| lrc.to_string_lossy().to_string());
    let has_lyrics =
        lrc_path.is_some() || tag.and_then(|t| t.get_string(&ItemKey::Lyrics)).is_some();

    // 封面:优先内嵌图片(同专辑只落盘一次),其次目录内的 cover/folder 图片
    let album_key = format!("{}\u{1}{}\u{1}{}", album_artist, album, parent.to_string_lossy());
    let cover = if let Some(c) = album_covers.get(&album_key) {
        Some(c.clone())
    } else {
        let mut found: Option<String> = None;
        if let Some(pic) = tag.and_then(|t| t.pictures().first()) {
            let ext = match pic.mime_type() {
                Some(MimeType::Png) => "png",
                _ => "jpg",
            };
            let file = covers_dir.join(format!("{:016x}.{ext}", hash_str(&album_key)));
            if fs::write(&file, pic.data()).is_ok() {
                found = Some(file.to_string_lossy().to_string());
            }
        }
        if found.is_none() {
            'outer: for name in COVER_NAMES {
                for ext in ["jpg", "jpeg", "png", "webp"] {
                    let p = parent.join(format!("{name}.{ext}"));
                    if p.is_file() {
                        found = Some(p.to_string_lossy().to_string());
                        break 'outer;
                    }
                }
            }
        }
        if let Some(f) = &found {
            album_covers.insert(album_key, f.clone());
        }
        found
    };

    Ok(Track {
        id: format!("{:016x}", hash_str(&path_str)),
        title,
        artist,
        album,
        album_artist,
        track_no,
        disc_no,
        year,
        genre,
        duration,
        path: path_str,
        cover,
        has_lyrics,
        lrc_path,
        added_at,
        mtime,
        size,
    })
}

fn parse_slashed_number(s: String) -> Option<u32> {
    s.split('/').next()?.trim().parse().ok()
}

fn hash_str(s: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}
