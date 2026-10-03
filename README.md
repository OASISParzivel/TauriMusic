# TauriMusic

参考 Apple Music 设计语言的本地音乐播放器,基于 **Rust + Tauri 2 + Vue 3** 构建。

![TauriMusic](https://img.shields.io/badge/Tauri-2-24C8D8) ![Rust](https://img.shields.io/badge/Rust-stable-DEA584) ![Vue](https://img.shields.io/badge/Vue-3-42B883)

## 功能特性

- **曲库扫描**:递归扫描指定的音乐文件夹,增量合并(文件未变化时跳过解析),扫描进度实时显示
- **元数据识别**:标题 / 艺人 / 专辑 / 专辑艺人 / 曲目号 / 年份 / 流派 / 时长,标签缺失时回退到 "艺人 - 标题" 文件名解析
- **封面识别**:优先提取内嵌封面(同一专辑只落盘一次),其次读取目录内的 `cover.*` / `folder.*` / `front.*` / `album.*` 图片
- **歌词识别**:
  - 内嵌歌词标签(ID3v2 USLT、Vorbis Comments `LYRICS` 等)
  - 同名 `.lrc` 伴生文件
  - 自动处理 GBK / UTF-16 编码的 LRC 文件,以及 "GBK 内容被标记为 Latin-1" 的乱码
  - 全屏播放页支持**逐行同步滚动**的歌词,点击歌词行跳转播放位置
- **播放**:底部播放条 + 全屏播放页,随机播放 / 列表循环 / 单曲循环,音量与进度记忆
- **全局快捷键**:空格 播放/暂停 · ←/→ 快退/快进 · Ctrl+←/→ 切歌 · ↑/↓ 音量 · M 静音 · Esc 关播放页(输入框聚焦时自动让位)
- **浏览视图**:最近添加、专辑(网格)、艺人(列表 + 详情)、歌曲、全文搜索(歌曲 / 专辑 / 艺人)
- **深浅色外观**:默认跟随系统,可手动切换
- **液态玻璃外观**:专辑封面驱动的氛围背景(深度模糊 + 缓慢漂移、随曲目切换淡入淡出),侧栏与播放条为悬浮磨砂玻璃卡片折射氛围色彩,可一键开关
- **TMC 音乐包**:`.tmc` = 标准 7z 包(音频 + 同名 LRC + 封面 + meta.json),支持双击导入 / 拖拽导入 / 一键导出,文件关联开箱即用
- **直接导入音乐文件**:设置页多选音频文件(或拖入窗口),自动通过网易云公开接口匹配封面与歌词
- **首次启动引导**:使用声明 + 感谢信两步式欢迎流程
- **关于页资源占用**:实时显示主进程内存与 CPU 占用

## 支持的格式

`mp3` `m4a` `flac` `ogg` `opus` `wav`(播放由系统 WebView2 解码器承担)

## 开发

```bash
npm install        # 安装前端依赖
npm run tauri dev  # 开发模式运行
```

要求:Node.js ≥ 18、Rust stable、WebView2(Windows 11 自带)。

## 构建

```bash
npm run tauri build             # 产出安装包(msi / nsis)
npm run tauri build -- --no-bundle  # 仅产出绿色版 exe
```

产物位于 `src-tauri/target/release/tauri-music.exe`。

## 生成示例曲库(可选)

生成一组带内嵌封面、内嵌歌词、伴生 LRC 与无标签文件的示例音乐到 `test-music/`,用于体验:

```bash
cd src-tauri
cargo run --example make_fixture
```

## 项目结构

```
src/                    # Vue 3 前端
  api.ts                #   Tauri command 封装与类型
  stores/               #   ui / library / player 三个响应式 store
  components/           #   侧边栏、播放条、全屏播放页、专辑卡片、曲目列表
  views/                #   最近添加 / 专辑 / 专辑详情 / 艺人 / 歌曲 / 搜索
src-tauri/
  src/scanner.rs        #   曲库扫描与元数据解析(lofty)
  src/lyrics.rs         #   LRC 解析与编码修复
  src/model.rs          #   数据模型与 library.json 持久化
  src/lib.rs            #   Tauri commands
  examples/make_fixture.rs  # 示例曲库生成器
```

## 数据存储

- 曲库与文件夹配置:`%APPDATA%\com.art3mis.taurimusic\library.json`
- 封面缓存:`%LOCALAPPDATA%\com.art3mis.taurimusic\covers\`

删除这两个目录即可完全重置应用。

## 许可

本项目基于 [Apache-2.0](LICENSE) 协议开源。
