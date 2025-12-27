# DBFS Persistence and Recovery Implementation

## Overview
This implementation replaces the previous "fake" in-memory persistence with a real block-device-backed solution using `jammdb`.

## Key Components

### 1. Persistence Adapter (`src/persistence.rs`)
The `persistence` module bridges the gap between `wrapper types` (like `BlockDevice`) and `jammdb`'s expected `File` interfaces.
- **BlockDeviceFile**: Implements `DbFile` trait. Forwards `read`, `write`, `seek` calls directly to the underlying `BlockDevice`.
- **BlockDeviceMapper**: Implements `MemoryMap` trait. Instead of memory-mapping a file (which isn't supported in `no_std` / bare metal), it provides a `PageLoader` that reads 4KB pages from the block device on demand.
- **Caching**: The adapter implements a basic page cache to satisfy `jammdb`'s requirement for reference stability (`&[u8]`).

### 2. High-Level Filesystem Logic (`src/dbfs_impl.rs`)
Since the upstream `dbfs2` crate's `Dbfs` struct was missing or relied on global state, a local `Dbfs` implementation was created.
- **Metadata Management**: Stores inodes and attributes in a `metadata` bucket.
- **Data Storage**: Stores file content in a `data` bucket using keys derived from inode and offset.
- **Directory Structure**: Uses a `dentry` bucket to map parent+name to child inodes.
- **Recovery/Format**: `Dbfs::new` calls `init_root()`. This checks if the `metadata` bucket exists. If not (fresh disk), it creates the root inode (formatting). If it exists, it assumes valid FS (recovery).

### 3. VFS Integration (`src/lib.rs`)
The `mount` function now:
1.  Extracts the `BlockDevice` from the input `VfsInode`.
2.  Initializes the `BlockDevicePersistence` layer.
3.  Opens `jammdb::DB` using this persistent backend.
4.  Passes the persistent `DB` to `Dbfs` for filesystem operations.

## Testing
A `MockBlockDevice` was implemented in `src/tests.rs` to verify persistence.
- **test_dbfs_persistence**:
    1.  Mounts a mock block device (RAM buffer).
    2.  Creates a file and writes data.
    3.  Unmounts (drops).
    4.  Re-mounts the SAME mock device.
    5.  Verifies the file and data exist.

> **Note**: Currently, `cargo test` execution is blocked by environment constraints (private dependencies). Verification will be performed once credentials or overrides are available.

## Limitations
- **Concurrency**: The page cache uses `Mutex` but relies on `unsafe` to satisfy lifetime requirements of `jammdb`. This works for the single-writer model of `jammdb` but requires care that cached pages are not evicted while in use by an active transaction.
- **Upstream Alignment**: The current implementation locally defines FS logic to bypass upstream limitations. Future work may involve realigning with improved `dbfs2` abstractions if they become available.
