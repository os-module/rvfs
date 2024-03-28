use std::{error::Error, sync::Arc};

use log::info;
use vfscore::{
    dentry::VfsDentry,
    fstype::VfsFsType,
    utils::{VfsNodePerm, VfsNodeType, VfsTimeSpec},
};

#[derive(Clone)]
pub struct RamFsProviderImpl;

impl ramfs::RamFsProvider for RamFsProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}

pub fn init_ramfs(ramfs: Arc<dyn VfsFsType>) -> Result<Arc<dyn VfsDentry>, Box<dyn Error>> {
    let root_dt = ramfs.i_mount(0, "/", None, &[])?;
    let root_inode = root_dt.inode()?;
    let f1 = root_inode.create(
        "f1.txt",
        VfsNodeType::File,
        VfsNodePerm::from_bits_truncate(0o666),
        None,
    )?;
    f1.write_at(0, b"hello world")?;
    let _f2 = root_inode.create(
        "f2.txt",
        VfsNodeType::File,
        VfsNodePerm::from_bits_truncate(0o666),
        None,
    )?;
    let _d1 = root_inode.create(
        "d1",
        VfsNodeType::Dir,
        VfsNodePerm::from_bits_truncate(0o755),
        None,
    )?;
    let _d2 = root_inode.create(
        "d2",
        VfsNodeType::Dir,
        VfsNodePerm::from_bits_truncate(0o755),
        None,
    )?;

    // root_dt.i_insert("f1.txt", f1.clone())?;
    // root_dt.i_insert("f2.txt", f2.clone())?;
    // root_dt.i_insert("d1", d1.clone())?;
    // root_dt.i_insert("d2", d2.clone())?;

    let f3 = root_inode.link("f3.txt", f1.clone())?;
    root_dt.i_insert("f3.txt", f3.clone())?;
    let f4 = root_inode.symlink("f4.txt", "f2.txt")?;
    root_dt.i_insert("f4.txt", f4.clone())?;

    info!("init ramfs");
    info!("ramfs tree:");
    info!(
        r"
    /
    ├── d1
    ├── d2
    ├── f1.txt
    ├── f2.txt
    ├── f3.txt
    └── f4.txt"
    );
    Ok(root_dt)
}
