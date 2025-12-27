# DBFS-VFS: Database-backed Filesystem VFS Adapter

A VFS (Virtual File System) adapter layer for database-backed filesystems, compatible with the `vfscore` traits.

## ⚠️ Current Implementation Status

**This is a reference implementation using in-memory storage.**

- ✅ **What it provides**: A complete VFS adapter demonstrating how to integrate database-backed storage with `vfscore`
- ✅ **What works**: All POSIX operations (read, write, create, lookup, readdir, unlink, truncate)
- ✅ **Thread-safe**: Uses `lock_api::Mutex` for concurrent access
- ❌ **What it lacks**: Persistent storage (currently uses `BTreeMap` in memory)

## Architecture

```text
┌─────────────────────────────────────┐
│   VFS Core Traits (vfscore)        │
│  (VfsInode, VfsFile, VfsDentry)    │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│      DBFS-VFS Adapter Layer         │
│  (DBFSInodeAdapter, DBFSDentry)     │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│   Storage Backend (pluggable)       │
│  Current: BTreeMap (in-memory)      │
│  Future: Real DBFS with persistence │
└─────────────────────────────────────┘
```

## Design Rationale

### Why In-Memory Storage?

This implementation uses `BTreeMap` as a temporary storage backend to:

1. **Demonstrate the VFS adapter pattern** without external dependencies
2. **Enable testing** of the VFS integration layer
3. **Provide a reference implementation** for future persistent backends
4. **Avoid coupling** to specific database or block device implementations

### Future Integration

The design is intentionally modular to support future integration with:

- **Real DBFS**: Persistent database-backed filesystem with block device
- **Alternative backends**: Any key-value store that fits the interface
- **Block device layer**: Direct integration with storage devices

## Storage Model

The current implementation uses a simple key-value model:

| Key Pattern | Value | Description |
|------------|-------|-------------|
| `i:{ino}` | Inode metadata | Stores inode number and basic attributes |
| `f:{ino}:0` | File data | File contents (chunk 0) |
| `d:{ino}` | Directory entries | Serialized list of child entries |

## Known Limitations

- [ ] **No persistence**: Data is lost when the process exits
- [ ] **No `.` and `..` entries**: `readdir` doesn't include POSIX-required dot entries
- [ ] **Simple serialization**: Uses basic binary encoding instead of proper database schema
- [ ] **Single chunk files**: Files are stored as a single chunk (no chunking for large files)
- [ ] **No block device integration**: Cannot interface with actual storage hardware

## Usage

```rust
use dbfs_vfs::{DBFSFs, DBFSProvider};
use vfscore::utils::VfsTimeSpec;

#[derive(Clone)]
struct MyProvider;

impl DBFSProvider for MyProvider {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}

// Create filesystem
let dbfs = DBFSFs::<_, spin::Mutex<()>>::new("my_db", MyProvider);

// Mount it
let root = dbfs.mount(0, "/", None, &[]).unwrap();

// Use it like any VFS
let root_inode = root.inode().unwrap();
let file = root_inode.create("test.txt", VfsNodeType::File, 
                              VfsNodePerm::from_bits_truncate(0o666), None).unwrap();
file.write_at(0, b"Hello, DBFS!").unwrap();
```

## Testing

Run the test suite:

```bash
cargo test -p dbfs-vfs
```

All tests should pass, demonstrating:
- Basic read/write operations
- Directory operations
- File lookup
- Directory listing
- File truncation
- File deletion

## Future Work

### Short-term (Next PR)
- [x] Add `.` and `..` entries to `readdir` for POSIX compliance
- [ ] Improve error handling and logging levels
- [ ] Add more comprehensive tests

### Long-term (Separate PRs)
- [ ] Replace `BTreeMap` with actual DBFS backend
- [ ] Integrate with block device interface
- [ ] Add persistence layer
- [ ] Implement proper chunking for large files
- [ ] Add crash recovery and consistency checks

## Contributing

When contributing to this crate, please keep in mind:

1. **Maintain the adapter pattern**: Keep storage backend pluggable
2. **Document limitations**: Be clear about what is/isn't implemented
3. **Test thoroughly**: All VFS operations should have tests
4. **Consider future integration**: Design with persistent storage in mind

## License

Same as the parent RVFS project.
