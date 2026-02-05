# Skill Kit

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
