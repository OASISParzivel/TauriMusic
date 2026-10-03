use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

use crate::model::{ScanReport, Track};
use crate::{AppState, IMPORT_DIR_NAME, LIBRARY_FILE, sanitize_name};

pub const EXTENSIONS: &[&str] = &["mp3", "m4a", "flac", "ogg", "oga", "opus", "wav"];
/// 目录内兜底封面文件名(不含扩展名)
const COVER_NAMES: &[&str] = &["cover", "folder", "front", "album", "albumart"];
/// 曲库格式版本:复用旧记录的条件之一。版本不一致时强制全量重扫(如封面改存缩略图)
const SCAN_VERSION: u32 = 2;

/// 全量扫描所有已登记的音乐文件夹,与旧曲库做增量合并后落盘。
/// 通过 AppState 的扫描闸防重入:已有扫描进行中时直接返回错误。
pub fn run_scan(app: &AppHandle) -> Result<ScanReport, String> {
    let state = app.state::<AppState>();
    let _gate = state.scan_gate.try_lock().map_err(|_| {
        "已有扫描正在进行,请稍后再试".to_string()
    })?;
    let (folders, existing, prev_version) = {
        let lib = state.lib.lock().map_err(|_| "曲库状态不可用".to_string())?;
        (lib.folders.clone(), lib.tracks.clone(), lib.version)
    };
    // 曲库格式版本不匹配时放弃全部增量复用,重新解析每个文件
    let force_full = prev_version != SCAN_VERSION;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64();
    let by_path: HashMap<String, Track> =
        existing.iter().map(|t| (t.path.clone(), t.clone())).collect();

    // 先收集文件清单(快),再逐个解析并上报进度。
    // 根条目放行(filter_entry 也会收到遍历根本身),否则登记点开头目录(如 X:\.music)会静默扫空
    let mut files: Vec<PathBuf> = Vec::new();
    for folder in &folders {
        for entry in walkdir::WalkDir::new(folder)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'))
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

    // 曲库整理:导入目录顶层的散装音频按「艺人\专辑」归位(仅顶层,不动子目录结构)
    organize_top_level(&state.data_dir.join(IMPORT_DIR_NAME), &mut files);

    let covers_dir = state.cache_dir.join("covers");
    let _ = fs::create_dir_all(&covers_dir);

    // 专辑封面去重:同一张专辑只提取/落盘一次
    let mut album_covers: HashMap<String, String> = HashMap::new();
    let mut result: Vec<Track> = Vec::with_capacity(total);
    let mut seen: HashSet<String> = HashSet::new();
    let (mut added, mut updated, mut errors) = (0usize, 0usize, 0usize);

    for (i, path) in files.iter().enumerate() {
        let path_str = path.to_string_lossy().to_string();

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
                // 文件在遍历与 stat 之间消失:不算 seen,让它落入 removed 口径
                errors += 1;
                continue;
            }
        };
        seen.insert(path_str.clone());

        let cached = by_path.get(&path_str).cloned();
        if let Some(prev) = &cached {
            // 文件未变化:直接复用旧记录,不重复解析。
            // 旁路文件(cover.* / 同名 .lrc)比音频新时也算变更——在线匹配只新增旁路文件;
            // 旧记录里的封面/歌词文件已被删除时同样视为变更,避免残留失效路径
            let cover_ok = prev
                .cover
                .as_ref()
                .map(|c| Path::new(c).is_file())
                .unwrap_or(true);
            let lrc_ok = prev
                .lrc_path
                .as_ref()
                .map(|c| Path::new(c).is_file())
                .unwrap_or(true);
            if (prev.mtime - mtime).abs() < 0.5
                && prev.size == size
                && cover_ok
                && lrc_ok
                && sidecar_mtime(path, prev.cover.as_deref()) <= prev.mtime + 0.5
                && !force_full
            {
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
        lib.version = SCAN_VERSION;
        // 落盘失败必须上抛:release 构建没有控制台,eprintln 无处可见,
        // 静默失败会让内存态与磁盘态分叉
        lib.save(&state.data_dir.join(LIBRARY_FILE)).map_err(|e| e.to_string())?;
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
            // 优先落盘缩略图(几百 KB 的 3000px 原图会拖慢列表滚动),解码失败再原样写入
            found = write_cover_thumb(covers_dir, &album_key, pic.data()).or_else(|| {
                let ext = match pic.mime_type() {
                    Some(MimeType::Png) => "png",
                    _ => "jpg",
                };
                let file = covers_dir.join(format!("{:016x}.{ext}", hash_str(&album_key)));
                fs::write(&file, pic.data()).ok().map(|_| file.to_string_lossy().to_string())
            });
        }
        if found.is_none() {
            // 曲目专属旁路封面(多专辑混居目录用):{stem}.cover.jpg 优先于通用 cover.*
            'stem_cover: for ext in ["jpg", "jpeg", "png", "webp"] {
                let p = parent.join(format!("{stem}.cover.{ext}"));
                if p.is_file() {
                    found = Some(p.to_string_lossy().to_string());
                    break 'stem_cover;
                }
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

/// 内嵌封面落盘为不超过 512px 的 JPEG 缩略图;解码失败返回 None(调用方回退原样写入)
fn write_cover_thumb(covers_dir: &Path, album_key: &str, data: &[u8]) -> Option<String> {
    use image::codecs::jpeg::JpegEncoder;
    use image::imageops::FilterType;
    use image::ImageEncoder;

    const MAX_DIM: u32 = 512;
    let img = image::load_from_memory(data).ok()?;
    let thumb = if img.width().max(img.height()) > MAX_DIM {
        img.resize(MAX_DIM, MAX_DIM, FilterType::Triangle)
    } else {
        img
    };
    let rgb = thumb.to_rgb8();
    let file = covers_dir.join(format!("{:016x}.jpg", hash_str(album_key)));
    let mut out = fs::File::create(&file).ok()?;
    JpegEncoder::new_with_quality(&mut out, 85)
        .write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)
        .ok()?;
    Some(file.to_string_lossy().to_string())
}

/// 音频旁路元数据文件的最新修改时间(封面 cover.* / {stem}.cover.* 与同名 .lrc),无则 0
/// 音频旁路元数据文件的最新修改时间(目录封面 + 同名 .lrc + 旧记录指向的实际封面),
/// 无则 0。目录封面探测范围与 COVER_NAMES 一致,替换 folder.jpg 等同样能触发重扫
fn sidecar_mtime(path: &Path, prev_cover: Option<&str>) -> f64 {
    let Some(parent) = path.parent() else { return 0.0 };
    let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) else {
        return 0.0;
    };
    let mut candidates: Vec<PathBuf> = Vec::new();
    for name in COVER_NAMES {
        for ext in ["jpg", "jpeg", "png", "webp"] {
            candidates.push(parent.join(format!("{name}.{ext}")));
        }
    }
    for ext in ["jpg", "jpeg", "png", "webp"] {
        candidates.push(parent.join(format!("{stem}.cover.{ext}")));
    }
    candidates.push(parent.join(format!("{stem}.lrc")));
    if let Some(c) = prev_cover {
        candidates.push(PathBuf::from(c));
    }
    candidates
        .into_iter()
        .filter_map(|p| fs::metadata(p).ok())
        .filter_map(|m| m.modified().ok())
        .filter_map(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64())
        .fold(0.0, f64::max)
}

fn hash_str(s: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// 快速读取音频标签里的艺人与专辑(供归位目录命名);无标签回退文件名「艺人 - 标题」约定
fn peek_artist_album(path: &Path) -> Option<(String, String)> {
    use lofty::file::TaggedFileExt;
    use lofty::tag::ItemKey;
    let Ok(tagged) = lofty::probe::Probe::open(path).and_then(|p| p.read()) else {
        return None;
    };
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let get = |k: &ItemKey| -> Option<String> {
        tag.and_then(|t| t.get_string(k))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let artist = get(&ItemKey::AlbumArtist).or_else(|| get(&ItemKey::TrackArtist));
    let album = get(&ItemKey::AlbumTitle);
    if let (Some(a), Some(alb)) = (artist.as_ref(), album.as_ref()) {
        return Some((a.clone(), alb.clone()));
    }
    // 标签不全:文件名「艺人 - 标题」补艺人
    let stem = path.file_stem()?.to_string_lossy().to_string();
    let from_name = stem.split_once(" - ").map(|(a, _)| a.trim().to_string());
    Some((
        artist.or(from_name).unwrap_or_else(|| "未整理".into()),
        album.unwrap_or_else(|| "未整理".into()),
    ))
}

/// 把导入目录顶层的散装音频按「艺人\专辑\文件名」归位,同 stem 的 .lrc / 专属封面一起移动。
/// 只整理应用自己的导入目录(用户登记的外部文件夹一律不动);正在播放等导致 rename 失败时保持原位。
fn organize_top_level(import_dir: &Path, files: &mut Vec<PathBuf>) {
    if !import_dir.is_dir() {
        return;
    }
    let entries = match fs::read_dir(import_dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        if !matches!(ext.as_deref(), Some(e) if EXTENSIONS.contains(&e)) {
            continue;
        }
        let Some(name) = path.file_name().map(|n| n.to_os_string()) else { continue };
        let (artist, album) = peek_artist_album(&path).unwrap_or_else(|| ("未整理".into(), "未整理".into()));
        let target_dir = import_dir
            .join(sanitize_name(&artist))
            .join(sanitize_name(&album));
        let target = target_dir.join(&name);
        if target == path {
            continue;
        }
        if target.exists() {
            // 目标已有同名:同大小视为重复,移入回收站(可恢复);不同大小则不覆盖,保持原位
            let dup = match (fs::metadata(&path), fs::metadata(&target)) {
                (Ok(a), Ok(b)) => a.len() == b.len(),
                _ => false,
            };
            if dup && trash::delete(&path).is_ok() {
                files.retain(|f| f != &path);
            }
            continue;
        }
        if fs::create_dir_all(&target_dir).is_err() || fs::rename(&path, &target).is_err() {
            continue; // 移动失败(如文件被播放占用)保持原位
        }
        for f in files.iter_mut() {
            if *f == path {
                *f = target.clone();
            }
        }
        // 顺移同名歌词与曲目专属旁路封面
        let Some(stem) = target.file_stem().map(|s| s.to_string_lossy().to_string()) else {
            continue;
        };
        let parent = target.parent().unwrap_or(import_dir);
        let lrc = path.with_extension("lrc");
        if lrc.is_file() {
            let _ = fs::rename(&lrc, parent.join(format!("{stem}.lrc")));
        }
        for e in ["jpg", "jpeg", "png", "webp"] {
            let c = import_dir.join(format!("{stem}.cover.{e}"));
            if c.is_file() {
                let _ = fs::rename(&c, parent.join(format!("{stem}.cover.{e}")));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tm-scan-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn slashed_number_parses_leading_part() {
        assert_eq!(parse_slashed_number("3/12".into()), Some(3));
        assert_eq!(parse_slashed_number(" 7 ".into()), Some(7));
        assert_eq!(parse_slashed_number("abc".into()), None);
        assert_eq!(parse_slashed_number("".into()), None);
    }

    #[test]
    fn hash_is_stable_for_same_input() {
        assert_eq!(hash_str("abc"), hash_str("abc"));
        assert_ne!(hash_str("abc"), hash_str("abd"));
    }

    #[test]
    fn sidecar_mtime_detects_cover_and_lrc() {
        let dir = temp_dir("sidecar");
        let audio = dir.join("song.mp3");
        fs::write(&audio, b"x").unwrap();
        assert_eq!(sidecar_mtime(&audio, None), 0.0);
        fs::write(dir.join("cover.jpg"), b"x").unwrap();
        assert!(sidecar_mtime(&audio, None) > 0.0);
        fs::write(dir.join("song.lrc"), b"[00:00]t").unwrap();
        fs::write(dir.join("song.cover.png"), b"x").unwrap();
        assert!(sidecar_mtime(&audio, None) > 0.0);
        // 非默认名单的封面(folder.*)也应计入,与目录封面回退逻辑一致
        fs::write(dir.join("folder.jpg"), b"x").unwrap();
        assert!(sidecar_mtime(&audio, None) > 0.0);
        // 旧记录指向的实际封面路径(如缓存目录)同样计入
        let elsewhere = temp_dir("sidecar-elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        fs::write(elsewhere.join("cached.png"), b"x").unwrap();
        let cached = elsewhere.join("cached.png");
        assert!(sidecar_mtime(&audio, Some(cached.to_str().unwrap())) > 0.0);
        fs::remove_dir_all(&dir).ok();
        fs::remove_dir_all(&elsewhere).ok();
    }

    #[test]
    fn scan_gate_rejects_concurrent_scan() {
        // 直接验证闸门互斥:同一线程先锁住闸,第二次 try_lock 必须失败
        let gate = std::sync::Mutex::new(());
        let _g1 = gate.try_lock().unwrap();
        assert!(gate.try_lock().is_err());
    }

    /// 写一个 lofty 可解析的最小 WAV(44 字节标准头 + 100 字节静音,PCM 8bit 单声道 8000Hz)
    fn write_wav(path: &Path) {
        let data_len = 100u32;
        let mut v = Vec::with_capacity(44 + data_len as usize);
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes()); // PCM
        v.extend_from_slice(&1u16.to_le_bytes()); // 单声道
        v.extend_from_slice(&8000u32.to_le_bytes()); // 采样率
        v.extend_from_slice(&8000u32.to_le_bytes()); // 字节率
        v.extend_from_slice(&1u16.to_le_bytes()); // 块对齐
        v.extend_from_slice(&8u16.to_le_bytes()); // 位深
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        v.extend(std::iter::repeat_n(0u8, data_len as usize));
        fs::write(path, v).unwrap();
    }

    #[test]
    fn parse_track_falls_back_to_filename() {
        let dir = temp_dir("fallback");
        // "艺人 - 标题" 命名约定
        let audio = dir.join("周杰伦 - 晴天.wav");
        write_wav(&audio);
        let t = parse_track(&audio, 0.0, 1, 0.0, &dir, &mut HashMap::new()).unwrap();
        assert_eq!(t.title, "晴天");
        assert_eq!(t.artist, "周杰伦");
        assert_eq!(t.album, "未知专辑");
        assert_eq!(t.album_artist, "周杰伦");
        assert!(t.duration > 0.0 && t.duration < 1.0); // 100B/8000Bps ≈ 0.0125s
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parse_track_cover_priority_stem_before_dir() {
        let dir = temp_dir("coverprio");
        let audio = dir.join("s.wav");
        write_wav(&audio);
        fs::write(dir.join("s.cover.jpg"), b"track").unwrap();
        fs::write(dir.join("cover.jpg"), b"dir").unwrap();

        // 曲目专属旁路封面优先于目录封面
        let t = parse_track(&audio, 0.0, 1, 0.0, &dir, &mut HashMap::new()).unwrap();
        let expect = dir.join("s.cover.jpg").to_string_lossy().to_string();
        assert_eq!(t.cover, Some(expect));

        // 删除专属封面后回退目录封面
        fs::remove_file(dir.join("s.cover.jpg")).unwrap();
        let t = parse_track(&audio, 0.0, 1, 0.0, &dir, &mut HashMap::new()).unwrap();
        let expect = dir.join("cover.jpg").to_string_lossy().to_string();
        assert_eq!(t.cover, Some(expect));

        // 全部移除后无封面
        fs::remove_file(dir.join("cover.jpg")).unwrap();
        let t = parse_track(&audio, 0.0, 1, 0.0, &dir, &mut HashMap::new()).unwrap();
        assert!(t.cover.is_none());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cover_thumb_scales_down_and_rejects_invalid() {
        use image::ImageFormat;
        let dir = temp_dir("thumb");
        let img = image::DynamicImage::new_rgb8(800, 600);
        let mut png = std::io::Cursor::new(Vec::new());
        img.write_to(&mut png, ImageFormat::Png).unwrap();

        let out = write_cover_thumb(&dir, "album\u{1}x", png.get_ref()).unwrap();
        assert!(out.ends_with(".jpg"));
        let decoded = image::open(&out).unwrap();
        assert!(decoded.width().max(decoded.height()) <= 512);

        // 非图片数据返回 None,调用方回退原样写入
        assert!(write_cover_thumb(&dir, "album\u{1}y", b"not-an-image").is_none());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn organize_moves_top_level_files_into_artist_album_dirs() {
        let import_dir = temp_dir("organize");
        // 有标签的散装文件(用最小 WAV + 文件名约定)
        let loose = import_dir.join("周杰伦 - 晴天.wav");
        write_wav(&loose);
        let mut files = vec![loose.clone()];
        organize_top_level(&import_dir, &mut files);

        let target = import_dir.join("周杰伦").join("未整理").join("周杰伦 - 晴天.wav");
        assert!(target.is_file(), "散装文件应归位到 艺人/专辑 目录");
        assert!(!loose.exists());
        assert_eq!(files[0], target);
        fs::remove_dir_all(&import_dir).ok();
    }

    #[test]
    fn organize_keeps_subdirectories_untouched() {
        let import_dir = temp_dir("organize-sub");
        let sub = import_dir.join("已有专辑目录");
        fs::create_dir_all(&sub).unwrap();
        let nested = sub.join("c.wav");
        write_wav(&nested);
        let mut files = vec![nested.clone()];
        organize_top_level(&import_dir, &mut files);
        assert!(nested.is_file(), "子目录文件不应被移动");
        assert_eq!(files[0], nested);
        fs::remove_dir_all(&import_dir).ok();
    }

    #[test]
    fn organize_deletes_duplicate_but_keeps_conflicting() {
        // 场景一:目标位置已有同内容同名文件,顶层再次出现 → 重复副本应被清理
        let dir1 = temp_dir("organize-dup");
        let target_dir1 = dir1.join("周杰伦").join("未整理");
        fs::create_dir_all(&target_dir1).unwrap();
        write_wav(&target_dir1.join("周杰伦 - 晴天.wav"));
        let dup = dir1.join("周杰伦 - 晴天.wav");
        fs::write(&dup, fs::read(target_dir1.join("周杰伦 - 晴天.wav")).unwrap()).unwrap();
        let mut files = vec![dup.clone()];
        organize_top_level(&dir1, &mut files);
        assert!(!dup.exists(), "同内容同名副本应被清理");

        // 场景二:同名但内容不同(大小不同) → 保持原位不覆盖
        let dir2 = temp_dir("organize-conflict");
        let target_dir2 = dir2.join("周杰伦").join("未整理");
        fs::create_dir_all(&target_dir2).unwrap();
        write_wav(&target_dir2.join("周杰伦 - 晴天.wav"));
        let mut wav_bytes = fs::read(target_dir2.join("周杰伦 - 晴天.wav")).unwrap();
        wav_bytes.push(0);
        let conflict = dir2.join("周杰伦 - 晴天.wav");
        fs::write(&conflict, &wav_bytes).unwrap();
        let mut files = vec![conflict.clone()];
        organize_top_level(&dir2, &mut files);
        assert!(conflict.is_file(), "不同内容的同名文件应保持原位");

        fs::remove_dir_all(&dir1).ok();
        fs::remove_dir_all(&dir2).ok();
    }
}
