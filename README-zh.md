# Git-Inner: A High-Performance Git Implementation in Rust

**项目定位：基于Rust语言实现的高性能、可扩展的类GitServer核心服务。**

`git-inner` 旨在提供一个现代化、高性能且安全的Git仓库管理核心，作为云原生开发平台和大规模代码托管服务的基石。通过利用Rust语言的内存安全和并发优势，我们致力于在性能和稳定性上超越现有解决方案。

---

## 目录
1. [项目目标](#项目目标)
2. [核心功能实现](#核心功能实现)
3. [开发环境配置](#开发环境配置)
4. [开发路线图](#开发路线图)
5. [API参考](#api参考)
6. [版本更新记录](#版本更新记录)

---

## 项目目标

### 1. 项目定位
本项目是一个用 Rust 从零开始构建的 Git 核心功能库和服务，其长期目标是成为一个功能完整、性能卓越的 Gitaly 替代品。它不仅会重新实现 Git 的核心对象模型和协议，还将提供强大的 RPC 接口，以便与 GitLab 等上层应用无缝集成。

### 2. 技术选型理由
- **Rust语言**: 提供了无GC的性能、内存安全和线程安全保障，这对于需要处理大量并发I/O和CPU密集型任务（如packfile压缩）的Git服务至关重要。
- **异步生态 (Tokio)**: 利用 `async/await` 语法，以极低的开销处理成千上万的并发连接和文件操作，是构建高性能网络服务的基础。
- **模块化设计**: 项目结构清晰，各模块（如 `objects`, `pack`, `refs`）高度解耦，便于独立开发、测试和维护。
- **可插拔后端**: `odb` 模块的设计允许未来支持多种存储后端（如文件系统、MongoDB、S3），以适应不同规模的部署需求。

---

## 核心功能实现

基于当前的项目结构，以下是各核心功能模块的开发状态和技术方案说明。

| 模块 | 主要功能 | 开发状态 | 技术实现方案 |
| :--- | :--- | :--- | :--- |
| **`sha`** | SHA-1/SHA-256 哈希计算 | ✅ 已完成 | 使用 `sha1` 和 `sha2` crate，为Git对象提供高效、准确的ID生成。 |
| **`objects`** | Git对象（Blob, Tree, Commit, Tag）的序列化与反序列化 | ✅ 已完成 | - 使用 `bytes` crate 进行零拷贝的二进制数据处理。<br>- 自定义解析器，严格遵循Git对象格式，性能优于通用序列化库。 |
| **`refs`** | 引用管理（branches, tags） | ✅ 已完成 | - loose refs: 直接读写 `.git/refs/` 目录下的文件。<br>- packed-refs: 解析和查询 `.git/packed-refs` 文件。 |
| **`odb`** | 对象数据库（Object Database） | 🧪 测试中 | - 默认实现为标准的 `objects` 目录文件系统后端。<br>- 实验性的 `odb/mongo` 后端，使用 `mongodb` crate 将Git对象存储在MongoDB中，探索分布式存储方案。 |
| **`pack`** | Packfile 的生成与解析 | 👨‍💻 开发中 | - 解析：实现 `idx` 和 `pack` 文件的解析，支持 delta-object 的重构。<br>- 生成：计划使用 `rayon` crate 并行计算 delta，提升压缩效率和速度。 |
| **`http`** | Git Smart HTTP 协议支持 | 🧪 测试中 | - 基于 `hyper` 和 `tokio` 构建异步HTTP服务。<br>- 实现 `info/refs` 和 `git-upload-pack`/`git-receive-pack` 的服务端点。 |
| **`transaction`** | 事务性引用更新 | 👨‍💻 开发中 | - 实现 `receive_pack` 核心逻辑，确保 `git push` 操作的原子性。<br>- 支持 pre-receive, update, post-receive 钩子。 |
| **`repository`** | 仓库的统一视图和管理 | 👨‍💻 开发中 | 提供上层API，封装对 `objects`, `refs`, `pack` 等底层模块的调用。 |
| **`hooks`** | Git 钩子 | ⏳ 待开发 | 设计钩子脚本的执行器，支持同步和异步钩子。 |

---

## 开发环境配置

1.  **安装 Rust**:
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
2.  **克隆项目**:
    ```bash
    git clone <repository_url>
    cd git-inner
    ```
3.  **构建与测试**:
    ```bash
    # 编译
    cargo build --release

    # 运行测试
    cargo test
    ```
4.  **依赖项**:
    - `openssl`
    - (可选) `mongodb` for `odb/mongo` backend.

---

## 开发路线图

### v0.2.0 - “读”操作与核心API稳定
- **优先级**:
    1.  [P0] 完成 `pack` 模块的完整解析功能。
    2.  [P0] 稳定 `repository` 模块的只读 API。
    3.  [P1] 完善 `http` 模块的 `git-upload-pack`（即 `git fetch`/`clone`）功能。
    4.  [P2] 建立初步的基准测试套件。

### v0.3.0 - “写”操作与初步RPC
- **优先级**:
    1.  [P0] 完成 `transaction/receive_pack` 模块，支持 `git push`。
    2.  [P0] 实现初步的 `pack` 文件生成能力。
    3.  [P1] 引入 `tonic` 和 `prost`，设计并实现第一版 RPC 接口（见下文）。
    4.  [P2] 实现 `pre-receive` 和 `post-receive` 钩子。

---


## API参考
完整的 Rust API 文档将通过 `cargo doc` 生成并托管。目前，请直接参考 `src` 目录下的代码注释。
*（此部分未来将链接到在线文档）*

---

## 版本更新记录
详细的版本历史和变更日志，请参见 [CHANGELOG.md](./CHANGELOG.md) 文件。