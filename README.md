# VFS
This crate provides a virtual file system implementation, which can be used in the kernel or user space.

The virtual file system is a file system abstraction layer. The virtual file system is responsible for managing all file systems, and all file system types must be registered in the virtual file system.


## Features
- [x] RamFs
- [x] DevFs
- [x] DynFs(It can be used as procfs/sysfs)
- [x] VfsCore
- [ ] ExtFs
- [ ] FatFs
- [ ] ...


## Demo
```bash
# run
RUST_LOG=info cargo run -p demo
```


## Usage
```
devfs = {path = "../devfs"}
ramfs = {path = "../ramfs"}
dynfs = {path = "../dynfs"}
vfscore = {path = "../vfscore"}
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

[Rust in the Linux. Ramfs in Rust | by Altimetrik Poland Tech Blog | Medium](https://altimetrikpoland.medium.com/rust-in-the-linux-e724ab4f1bad)

[linux的VFS详解_vfs posix_土豆西瓜大芝麻的博客-CSDN博客](https://blog.csdn.net/jinking01/article/details/90669534)

[spinlock.pdf (slideshare.net)](https://www.slideshare.net/AdrianHuang/spinlockpdf)

[Overview of the Linux Virtual File System — The Linux Kernel documentation](https://docs.kernel.org/filesystems/vfs.html)