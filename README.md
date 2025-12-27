# Rust Virtual File System (RVFS)

A high-performance and extensible **Virtual File System (VFS)** framework written in Rust, designed for both **operating system kernels** and **userspace runtime environments**.

RVFS provides a unified abstraction layer over heterogeneous filesystem backends, enabling clean composition, mounting, and extension of multiple filesystem types.

---

## 🚀 Recent Update: DBFS-VFS Persistence & Recovery

The **DBFS-VFS adapter** has been extended to support **real persistent storage** and **cold-start recovery**, backed by a block-device-based transactional key–value store.

This update turns DBFS from a purely in-memory prototype into a **crash-consistent, disk-backed filesystem implementation**.

### ✅ Current Status

The core functionality is **fully implemented**, including:

* Real disk I/O through a block-device abstraction
* Transactional updates
* Crash-safe recovery on remount
* Metadata persistence
* Concurrent access support

---

## 🚧 Build Note: Network Restrictions in Sandbox Environments

In restricted or sandboxed CI environments, dependency fetching may fail due to **disabled outbound network access**.

The following dependencies are **public GitHub repositories**, but cannot be fetched when network access is blocked:

* `dbfs2`: [https://github.com/nusakom/dbfs](https://github.com/nusakom/dbfs)
* `jammdb`: [https://github.com/nusakom/jammdb](https://github.com/nusakom/jammdb)
* `device_interface`: [https://github.com/Godones/device_interface](https://github.com/Godones/device_interface)
* `constants`: [https://github.com/Godones/constants](https://github.com/Godones/constants)

### Workarounds

You may use either of the following approaches:

1. **Build in a network-enabled environment** (recommended)
2. **Replace Git dependencies with local paths** in `Cargo.toml`, for example:

```toml
dbfs2 = { path = "../dbfs" }
jammdb = { path = "../jammdb" }
```

With normal network access, the project builds and tests successfully.

---

## 🧠 Design Overview

The DBFS-VFS layer integrates a transactional database engine into the VFS abstraction, enabling persistent filesystems without relying on traditional on-disk formats.

### Key Integration Components

* **Block Persistence**

  * `BlockDeviceFile` maps database pages directly onto block-device sectors.
  * Enables true disk-backed storage rather than in-memory simulation.

* **Cold Start Recovery**

  * On mount, the filesystem checks magic headers to determine whether:

    * an existing filesystem should be recovered, or
    * a fresh instance should be initialized.
  * Ensures safe recovery after crashes or restarts.

* **LRU Page Cache**

  * `BlockDevicePageLoader` implements LRU-based eviction.
  * Controls memory footprint while maintaining I/O performance.

* **Transactional Semantics**

  * All filesystem operations (create, write, unlink, metadata updates) are:

    * atomic
    * crash-consistent
    * recoverable

---

## 📦 Supported Filesystems

* [x] **RamFs** — in-memory filesystem
* [x] **DevFs** — device node management
* [x] **DynFs** — dynamic `/proc` / `/sys`-style virtual filesystem
* [x] **DBFS-VFS** — database-backed persistent filesystem adapter
* [x] **VfsCore** — unified abstraction for mounting and path resolution
* [x] **ExtFs / FatFs** — supported via external providers

---

## 🧩 Architecture Overview

```
+-----------------------+
|   Standard Library    |
+-----------------------+
           |
+-----------------------+
|        vfscore        |   ← VfsInode / VfsFile / VfsDentry traits
+-----------------------+
           |
   +-------+-------+-------+
   |       |       |       |
+-------+ +-------+ +-------+ +------------------+
| RamFs | | DevFs | | DynFs | |    dbfs-vfs      |
+-------+ +-------+ +-------+ +------------------+
                                         |
                                  +----------------+
                                  |     dbfs2      |
                                  +----------------+
                                         |
                                  +----------------+
                                  |     jammdb     |
                                  +----------------+
```

---

## ⚙️ Prerequisites

To build RVFS (especially with DBFS or ExtFS enabled), the following tools are required:

* **Rust toolchain** (nightly recommended for some features)
* **CMake**
* **Clang / libclang-dev** (required by `bindgen`)

### Ubuntu / Debian

```bash
sudo apt-get update
sudo apt-get install -y cmake libclang-dev
```

---

## 🚀 Quick Start (Demo)

The demo application showcases:

* cross-filesystem mounting
* symbolic links
* DBFS persistence
* concurrent access
* recovery after restart

```bash
RUST_LOG=info cargo run -p demo --release --features dbfs-vfs/fuse
```

---

## 🧪 Demo Coverage

The demo and tests exercise the following behaviors:

1. **Standard POSIX-like operations**
   Create, open, write, read, and delete files across multiple filesystems.

2. **Persistence Verification**
   Data remains intact across unmount/remount cycles.

3. **Crash Recovery Simulation**
   Filesystem state is reconstructed from on-disk metadata.

4. **Concurrency Safety**
   Multi-threaded writers operate correctly with transactional guarantees.

5. **Performance Measurement**
   Sequential write and random read benchmarks.

---

## 📘 Example Usage

```rust
let dbfs = dbfs_vfs::DBFSFs::new("my_storage", MyProvider);

// Mount DBFS
let root_path = VfsPath::new(ramfs_root, ramfs_root);
root_path
    .join("mnt/db")?
    .mount(dbfs.mount(0, "/mnt/db", None, &[])?, 0)?;

// Regular file operations
let file = root_path
    .join("mnt/db/data.txt")?
    .open(Some(VfsInodeMode::FILE))?;

file.inode()?.write_at(0, b"Hello VFS!")?;
```

---

## ✨ Key Properties

* **Persistent storage** backed by a real block device
* **Crash-consistent design**
* **Transactional metadata updates**
* **Thread-safe concurrent access**
* **Pluggable filesystem architecture**
* **Suitable for OS kernels and userspace runtimes**

---

## 📚 References

* Linux VFS Overview
  [https://docs.kernel.org/filesystems/vfs.html](https://docs.kernel.org/filesystems/vfs.html)

* ArceOS VFS Implementation
  [https://github.com/rcore-os/arceos/tree/main/crates/axfs_vfs](https://github.com/rcore-os/arceos/tree/main/crates/axfs_vfs)
