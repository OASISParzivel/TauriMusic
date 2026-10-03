use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 一首曲目。字段名经 serde 转换后与前端 TypeScript 接口一一对应。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    /// 时长(秒)
    pub duration: f64,
    /// 音频文件绝对路径
    pub path: String,
    /// 缓存后的封面图片绝对路径(内嵌封面已落盘,或专辑目录内的 cover/folder 图片)
    pub cover: Option<String>,
    /// 是否存在可用歌词(内嵌或同名 .lrc)
    pub has_lyrics: bool,
    /// 同名 .lrc 伴生文件路径
    pub lrc_path: Option<String>,
    /// 首次入库时间(unix 秒)
    pub added_at: f64,
    /// 文件修改时间(unix 秒),用于增量扫描
    pub mtime: f64,
    pub size: u64,
}

/// 整个曲库,持久化为 app_data/library.json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Library {
    pub version: u32,
    pub folders: Vec<String>,
    pub tracks: Vec<Track>,
}

impl Library {
    pub fn load(path: &Path) -> Library {
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        fs::write(path, json)
    }
}

/// 扫描结果报告
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
    pub total: usize,
    pub errors: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_save_load_roundtrip() {
        let lib = Library {
            version: 2,
            folders: vec!["F:\\music".into()],
            tracks: vec![Track {
                id: "abc123".into(),
                title: "晴天".into(),
                artist: "周杰伦".into(),
                album: "叶惠美".into(),
                track_no: Some(3),
                duration: 269.2,
                path: "F:\\music\\晴天.mp3".into(),
                mtime: 1759000000.5,
                size: 1024,
                ..Default::default()
            }],
        };
        let path = std::env::temp_dir().join(format!("tm-lib-{}.json", std::process::id()));
        lib.save(&path).unwrap();
        let back = Library::load(&path);
        assert_eq!(back.version, 2);
        assert_eq!(back.folders, lib.folders);
        assert_eq!(back.tracks[0].title, "晴天");
        assert_eq!(back.tracks[0].track_no, Some(3));
        assert_eq!(back.tracks[0].mtime, 1759000000.5);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn library_load_missing_file_returns_default() {
        let lib = Library::load(Path::new("Z:\\nonexistent\\tm-lib.json"));
        assert!(lib.tracks.is_empty());
        assert_eq!(lib.version, 0);
    }
}
