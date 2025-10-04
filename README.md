# Git-Inner: A High-Performance Git Implementation in Rust

**Project Positioning: A high-performance, scalable, GitServer-like core service implemented in Rust.**

`git-inner` aims to provide a modern, high-performance, and secure Git repository management core, serving as a cornerstone for cloud-native development platforms and large-scale code hosting services. By leveraging the memory safety and concurrency advantages of the Rust language, we are committed to surpassing existing solutions in performance and stability.

---

## Table of Contents
1. [Project Goals](#project-goals)
2. [Core Feature Implementation](#core-feature-implementation)
3. [Development Environment Setup](#development-environment-setup)
4. [Development Roadmap](#development-roadmap)
5. [API Reference](#api-reference)
6. [Version History](#version-history)

---

## Project Goals

### 1. Project Positioning
This project is a Git core functional library and service built from scratch in Rust. Its long-term goal is to become a feature-complete and high-performance alternative to Gitaly. It will not only reimplement Git's core object model and protocols but also provide powerful RPC interfaces for seamless integration with upstream applications like GitLab.

### 2. Rationale for Technology Choices
- **Rust Language**: Provides GC-free performance, memory safety, and thread safety, which are crucial for a Git service that needs to handle a large number of concurrent I/O and CPU-intensive tasks (like packfile compression).
- **Asynchronous Ecosystem (Tokio)**: Utilizes `async/await` syntax to handle thousands of concurrent connections and file operations with extremely low overhead, forming the foundation for building high-performance network services.
- **Modular Design**: The project has a clear structure with highly decoupled modules (e.g., `objects`, `pack`, `refs`), facilitating independent development, testing, and maintenance.
- **Pluggable Backend**: The design of the `odb` module allows for future support of multiple storage backends (e.g., file system, MongoDB, S3) to adapt to different deployment scales.

---

## Core Feature Implementation

Based on the current project structure, here is the development status and technical implementation plan for each core functional module.

| Module | Main Function | Development Status | Technical Implementation Plan |
| :--- | :--- | :--- | :--- |
| **`sha`** | SHA-1/SHA-256 Hash Calculation | ✅ Completed | Use `sha1` and `sha2` crates for efficient and accurate ID generation for Git objects. |
| **`objects`** | Serialization & Deserialization of Git Objects (Blob, Tree, Commit, Tag) | ✅ Completed | - Use the `bytes` crate for zero-copy binary data handling.<br>- Custom parser strictly follows the Git object format for better performance than generic serialization libraries. |
| **`refs`** | Reference Management (branches, tags) | ✅ Completed | - loose refs: Direct read/write of files under the `.git/refs/` directory.<br>- packed-refs: Parse and query the `.git/packed-refs` file. |
| **`odb`** | Object Database | 🧪 Testing | - Default implementation is a standard file system backend in the `objects` directory.<br>- Experimental `odb/mongo` backend using the `mongodb` crate to store Git objects in MongoDB, exploring distributed storage solutions. |
| **`pack`** | Packfile Generation & Parsing | 👨‍💻 In Development | - Parsing: Implement parsing for `idx` and `pack` files, supporting delta-object reconstruction.<br>- Generation: Plan to use the `rayon` crate for parallel delta computation to improve compression efficiency and speed. |
| **`http`** | Git Smart HTTP Protocol Support | 🧪 Testing | - Build an asynchronous HTTP service based on `hyper` and `tokio`.<br>- Implement server endpoints for `info/refs` and `git-upload-pack`/`git-receive-pack`. |
| **`transaction`** | Transactional Reference Updates | 👨‍💻 In Development | - Implement the core logic of `receive_pack` to ensure atomicity of `git push` operations.<br>- Support for pre-receive, update, and post-receive hooks. |
| **`repository`** | Unified Repository View and Management | 👨‍💻 In Development | Provide a high-level API to encapsulate calls to underlying modules like `objects`, `refs`, `pack`, etc. |
| **`hooks`** | Git Hooks | ⏳ To Be Developed | Design an executor for hook scripts, supporting both synchronous and asynchronous hooks. |

---

## Development Environment Setup

1.  **Install Rust**:
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
2.  **Clone the Project**:
    ```bash
    git clone <repository_url>
    cd git-inner
    ```
3.  **Build and Test**:
    ```bash
    # Compile
    cargo build --release

    # Run tests
    cargo test
    ```
4.  **Dependencies**:
    - `openssl`
    - (Optional) `mongodb` for `odb/mongo` backend.

---

## Development Roadmap

### v0.2.0 - "Read" Operations & Core API Stabilization
- **Priorities**:
    1.  [P0] Complete the full parsing functionality of the `pack` module.
    2.  [P0] Stabilize the read-only API of the `repository` module.
    3.  [P1] Enhance the `git-upload-pack` (i.e., `git fetch`/`clone`) functionality in the `http` module.
    4.  [P2] Establish an initial benchmark test suite.

### v0.3.0 - "Write" Operations & Initial RPC
- **Priorities**:
    1.  [P0] Complete the `transaction/receive_pack` module to support `git push`.
    2.  [P0] Implement initial packfile generation capabilities.
    3.  [P1] Introduce `tonic` and `prost` to design and implement the first version of the RPC interface.
    4.  [P2] Implement `pre-receive` and `post-receive` hooks.

---


## API Reference
The complete Rust API documentation will be generated via `cargo doc` and hosted online. For now, please refer to the code comments in the `src` directory.
*(This section will link to the online documentation in the future.)*

---

## Version History
For a detailed version history and changelog, please see the [CHANGELOG.md](./CHANGELOG.md) file.