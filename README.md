# MCL Launcher

用 Tauri 2 + React + TypeScript 编写的本地 Minecraft 启动器初版。当前完成第 1～3 步：

1. Tauri 2 + React 骨架、SQLite 建表、路由与基础布局。
2. Mojang 版本清单读取、缓存、筛选与版本列表 UI。
3. Vanilla 客户端、依赖库和资源下载：并发、进度事件、重试、Range 断点续传、SHA-1 校验、BMCLAPI 镜像切换。

## 技术栈

- Tauri 2 / Rust 1.99
- React 19 / TypeScript 6 / Vite 8
- Tailwind CSS 4 / Zustand 5 / React Router 7
- SQLite（rusqlite，bundled）
- reqwest + tokio
- zip

## 本地目录

默认数据目录为 `~/.launcher`，也可以通过 `MCL_HOME` 环境变量覆盖。

```text
~/.launcher/
├─ launcher.db
├─ instances/<instance-id>/
│  ├─ mods/
│  ├─ config/
│  ├─ saves/
│  ├─ logs/
│  ├─ crash-reports/
│  ├─ resourcepacks/
│  └─ shaderpacks/
└─ shared/
   ├─ versions/<version-id>/
   │  ├─ version.json
   │  ├─ client.jar
   │  ├─ natives/
   │  └─ .mcl-installed.json
   ├─ libraries/
   ├─ assets/
   │  ├─ indexes/
   │  └─ objects/
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
```

## 当前功能

- 新建、查看、删除实例。
- 每个实例有独立的 `mods/`、`config/`、`saves/` 等目录。
- 版本清单读取和缓存。
- 正式版、快照、远古版筛选与版本号搜索。
- 一键安装 Vanilla 游戏文件到共享缓存。
- 安装阶段实时进度、文件计数、下载字节数和百分比。
- 官方源和 BMCLAPI 镜像切换。
- 下载失败重试，`.part` 文件断点续传，完成后校验 SHA-1。
- 设置页保存镜像与并发下载数。
- 实例详情页可修改内存和 JVM 参数。

## 已完成的第 1～3 步对应模块

```text
src-tauri/src/
├─ commands/          Tauri command 入口
├─ core/
│  ├─ instance.rs     实例 CRUD 与目录隔离
│  ├─ version.rs      版本清单与缓存
│  ├─ install.rs      下载计划和安装器
│  ├─ paths.rs        全局目录规则
│  └─ loader.rs       Fabric/Quilt/NeoForge/Forge 预留
└─ db/
   ├─ schema.rs       SQLite 迁移和默认设置
   └─ models.rs       数据模型
```

## 说明

- 第 4～11 步尚未实现：Java 检测、离线/微软登录、启动游戏、加载器安装、模组管理和完整日志窗口。
- `libraries/` 与 `assets/` 为全局共享缓存；实例只保存自己的配置、存档和模组。
- 所有操作都在本机完成，不上传账户、实例或游戏数据。
