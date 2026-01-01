# VFS
This crate provides a virtual file system implementation, which can be used in the kernel or user space.

The virtual file system is a file system abstraction layer. The virtual file system is responsible for managing all file systems, and all file system types must be registered in the virtual file system.


## Features
- [x] RamFs
- [x] DevFs
- [x] DynFs(It can be used as procfs/sysfs)
- [x] VfsCore
- [x] ExtFs
- [x] FatFs
- [ ] ...


## Demo
```bash
# run
RUST_LOG=info cargo run -p demo
```


## Usage
```
devfs = { git = "https://github.com/nusakom/rvfs" }
ramfs = { git = "https://github.com/nusakom/rvfs" }
dynfs = { git = "https://github.com/nusakom/rvfs" }
fat-vfs = { git = "https://github.com/nusakom/rvfs" }
lwext-vfs = { git = "https://github.com/nusakom/rvfs" }
vfscore = { git = "https://github.com/nusakom/rvfs" }
```
```rust
// create a fs_type
let ramfs = Arc::new(RamFs::<_, Mutex<()>>::new(RamFsProviderImpl));
// create a fs instance
let root_dt = ramfs.i_mount(MountFlags::empty(), None, &[])?;
// get the inode
let root_inode = root_dt.inode()?;
// create a file
let f1 = root_inode.create(
    "f1.txt",
    VfsNodeType::File,
    VfsNodePerm::from_bits_truncate(0o666),
    None,
)?;
```



## Reference

[Overview of the Linux Virtual File System — The Linux Kernel documentation](https://docs.kernel.org/filesystems/vfs.html)

https://github.com/rcore-os/arceos/tree/main/crates/axfs_vfs

https://github.com/yfblock/ByteOS

