use std::error::Error;
use std::sync::Arc;
use log::info;
use vfscore::dentry::VfsDentry;
use vfscore::fstype::{MountFlags, VfsFsType};
use vfscore::utils::{VfsNodePerm, VfsNodeType, VfsTimeSpec};

#[derive(Clone)]
pub struct RamFsProviderImpl;

impl ramfs::KernelProvider for RamFsProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}

pub fn init_ramfs(ramfs:Arc<dyn VfsFsType>) -> Result<Arc<dyn VfsDentry>, Box<dyn Error>> {
    let root_dt = ramfs.clone().mount(MountFlags::empty(), "", &[])?;
    let root_inode=  root_dt.inode()?;
    let f1 = root_inode.create("f1.txt", VfsNodeType::File, VfsNodePerm::from_bits_truncate(0o666), None)?;
    let f2 = root_inode.create("f2.txt", VfsNodeType::File, VfsNodePerm::from_bits_truncate(0o666), None)?;
    let d1 = root_inode.create("d1", VfsNodeType::Dir, VfsNodePerm::from_bits_truncate(0o755), None)?;
    let d2 = root_inode.create("d2", VfsNodeType::Dir, VfsNodePerm::from_bits_truncate(0o755), None)?;

    root_dt.clone().insert("f1.txt",f1.clone())?;
    root_dt.clone().insert("f2.txt",f2.clone())?;
    root_dt.clone().insert("d1",d1.clone())?;
    root_dt.clone().insert("d2",d2.clone())?;

    let f3 = root_inode.link("f3.txt",f1.clone())?;
    root_dt.clone().insert("f3.txt",f3.clone())?;
    let f4 = root_inode.symlink("f4.txt","f2.txt")?;
    root_dt.clone().insert("f4.txt",f4.clone())?;
    
    info!("init ramfs");
    info!("ramfs tree:");
    info!(r"
    /
    ├── d1
    ├── d2
    ├── f1.txt
    ├── f2.txt
    ├── f3.txt
    └── f4.txt");
    Ok(root_dt)
}
