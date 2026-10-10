# EMCL — Ember Minecraft Launcher

EMCL 是一个用 Tauri 2 + React + TypeScript 编写的本地 Minecraft 启动器。E 代表 **Ember**，界面采用无边框窗口和自定义标题栏。

当前已覆盖计划中的第 1～11 步：

1. Tauri 2 + React 骨架、SQLite 建表、路由与基础布局。
2. Mojang 版本清单读取、缓存、筛选与版本列表 UI。
3. Vanilla 客户端、依赖库和资源下载：并发、进度事件、重试、Range 断点续传、SHA-1 校验、BMCLAPI 镜像切换。
4. Java 检测、版本推荐和实例/全局选择。
5. 实例 CRUD、目录隔离和实例详情配置。
6. 离线账户、标准离线 UUID 和账户切换。
7. Vanilla 启动命令组装、`tokio::process` 进程管理、实时 stdout/stderr 日志。
8. Microsoft 设备码登录、Xbox Live/XSTS、Minecraft 服务和刷新令牌。
9. Fabric、Quilt、NeoForge、Forge 加载器安装。
10. 模组拖拽安装、启停、删除和元数据解析。
11. 全局设置与实例设置，包括内存、JVM、Java、窗口尺寸、全屏和下载源。

## 技术栈

- Tauri 2 / Rust 1.99
- React 19 / TypeScript 6 / Vite 8
- Tailwind CSS 4 / Zustand 5 / React Router 7
- SQLite（rusqlite，bundled）
- reqwest + tokio
- zip / toml / quick Maven 元数据解析

## 本地目录

默认数据目录为 `~/.emcl`，也可以通过 `EMCL_HOME` 环境变量覆盖。

```text
~/.emcl/
├─ launcher.db
├─ instances/<instance-id>/
│  ├─ mods/
│  ├─ config/
│  ├─ saves/
│  ├─ logs/latest.log
│  ├─ crash-reports/
│  ├─ resourcepacks/
│  └─ shaderpacks/
└─ shared/
   ├─ versions/<version-id>/
   │  ├─ version.json
   │  ├─ client.jar
   │  ├─ natives/
   │  └─ .emcl-installed.json
   ├─ libraries/
   ├─ assets/
   │  ├─ indexes/
   │  └─ objects/
   ├─ loaders/
   └─ log_configs/
```

## 开发与构建

```powershell
pnpm install
pnpm tauri dev
```

生产构建：

```powershell
pnpm build
pnpm tauri build
```

只构建可执行文件、不生成安装包：

```powershell
pnpm tauri build --no-bundle
```

## 桌面快捷方式

构建后可运行：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\create-desktop-shortcut.ps1
```

脚本会在桌面创建 `EMCL.lnk`，指向 `src-tauri/target/release/emcl-launcher.exe`。

## 验证项目

```powershell
pnpm check
pnpm build
```

```powershell
cd src-tauri
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test parses_1_21_1_metadata -- --ignored
cargo test downloads_real_asset_with_checksum -- --ignored
```

## Microsoft 登录

Microsoft 登录需要 Azure 公共客户端应用 ID。进入设置页的“Microsoft 登录”填入 Application (client) ID；留空时只禁用微软登录，不影响离线账户。

## 目录结构

```text
src-tauri/src/
├─ commands/          Tauri command 入口
├─ core/
│  ├─ auth.rs         离线账户与 Microsoft 登录
│  ├─ instance.rs     实例 CRUD 与目录隔离
│  ├─ version.rs      版本清单、参数和缓存
│  ├─ install.rs      下载计划和安装器
│  ├─ java.rs         Java 检测与推荐
│  ├─ launch.rs       启动命令、进程管理和日志
│  ├─ loader.rs       Fabric/Quilt/NeoForge/Forge
│  ├─ mods.rs         模组安装与元数据
│  └─ paths.rs        全局目录规则
└─ db/
   ├─ schema.rs       SQLite 迁移和默认设置
   └─ models.rs       数据模型
```

## 说明

- `libraries/` 与 `assets/` 为全局共享缓存；实例只保存自己的配置、存档和模组。
- 启动日志会实时推送到 GUI，同时写入实例的 `logs/latest.log`。
- 所有账户和实例数据都只保存在本机，不上传任何数据。
- NeoForge/Forge 使用官方 installer 安装并复制生成的版本 profile 和依赖库。
