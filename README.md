# YouSkill

一个基于 Tauri + Svelte 的桌面端技能管理工具。

## 功能特性

- **本地技能管理** - 扫描、安装和管理本地技能包
- **远程技能市场** - 浏览和安装远程技能
- **多语言支持** - 支持中文和英文界面切换
- **主题切换** - 支持浅色/深色/系统主题
- **多种同步模式** - 支持符号链接和复制两种同步方式

## 技术栈

- **Tauri v2** - Rust 构建的轻量级桌面应用框架
- **Svelte 5** - 响应式前端框架
- **Vite** - 下一代前端构建工具
- **Tailwind CSS** - 实用优先的 CSS 框架
- **Lucide Icons** - 优雅的图标库

## 开发环境

### 前置要求

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri CLI](https://tauri.app/start/prerequisites/)

### 安装依赖

```bash
npm install
```

### 启动开发服务器

```bash
npm run tauri:dev
```

### 构建生产版本

```bash
npm run tauri:build
```

## 下载安装

从 [Releases](https://github.com/YOUR_USERNAME/you-skill/releases) 页面下载对应平台的安装包：

- **macOS**: `YouSkill_0.1.0_aarch64.dmg` (Apple Silicon) / `YouSkill_0.1.0_x64.dmg` (Intel)
- **Windows**: `YouSkill_0.1.0_x64-setup.exe`
- **Linux**: `you-skill_0.1.0_amd64.deb` / `you-skill-0.1.0-1.x86_64.rpm`

> **注意**: macOS 版本需要 macOS 10.13 或更高版本。首次打开时可能需要前往 **系统设置 > 隐私与安全性** 允许运行。

## 发布新版本

### 使用发布脚本（推荐）

```bash
# 发布新版本（自动更新版本号、CHANGELOG、提交、打标签、推送）
./scripts/release.sh 0.2.0 "添加新功能描述"

# 或者只更新版本号，手动推送
./scripts/release.sh 0.2.0 "添加新功能描述"
# 然后选择不推送，手动执行：
# git push origin main
# git push origin v0.2.0
```

脚本会自动：
1. 更新 `package.json`、`tauri.conf.json`、`Cargo.toml` 的版本号
2. 更新 `CHANGELOG.md`
3. 提交更改
4. 创建带注释的 Git 标签
5. 推送到远程仓库

### 手动发布

如果不想使用脚本，可以手动操作：

1. 更新以下文件的版本号：
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
2. 更新 [CHANGELOG.md](CHANGELOG.md)
3. 提交更改：`git commit -m "chore: bump version to 0.2.0"`
4. 创建标签：`git tag -a v0.2.0 -m "Release v0.2.0"`
5. 推送：`git push origin main && git push origin v0.2.0`

GitHub Actions 会自动构建并创建 Release。

详见 [SIGNING.md](SIGNING.md) 了解代码签名配置。

## 项目结构

```
src/
├── lib/
│   ├── components/     # Svelte 组件
│   │   ├── PageHeader.svelte
│   │   ├── LocalSkillsSection.svelte
│   │   ├── RemoteSkillsSection.svelte
│   │   ├── SettingsPanel.svelte
│   │   ├── AddSkillModal.svelte
│   │   └── IconButton.svelte
│   ├── stores/         # Svelte stores (状态管理)
│   └── i18n/           # 国际化配置
├── App.svelte          # 根组件
└── main.js             # 入口文件

src-tauri/
├── src/                # Rust 源代码
├── icons/              # 应用图标
└── tauri.conf.json     # Tauri 配置
```

## 配置说明

应用设置存储在系统中，可通过设置面板修改：

- **语言** - 切换界面语言 (en/zh)
- **主题** - 选择界面主题 (system/light/dark)
- **同步模式** - 技能安装方式 (symlink/copy)

## 许可证

MIT
