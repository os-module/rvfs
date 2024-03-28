use std::{cmp::min, error::Error, sync::Arc};

use dynfs::{DynFs, DynFsDirInode, DynFsKernelProvider};
use spin::Mutex;
use vfscore::{
    error::VfsError, file::VfsFile, fstype::VfsFsType, inode::VfsInode, path::DirIter, utils::*,
    VfsResult,
};

#[derive(Clone)]
struct DynFsKernelProviderImpl;

impl DynFsKernelProvider for DynFsKernelProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let procfs = Arc::new(DynFs::<_, Mutex<()>>::new(
        DynFsKernelProviderImpl,
        "procfs",
    ));
    let root_dt = procfs.clone().mount(0, "/", None, &[])?;
    let root_inode = root_dt.inode()?;

    // Procfs don't support to create file/dir at runtime
    assert!(root_inode
        .create("1", VfsNodeType::File, "r--r--r--".into(), None)
        .is_err());
    assert!(root_inode
        .create("1", VfsNodeType::Dir, "r--r--r--".into(), None,)
        .is_err());

    type DynFsDirInodeImpl = DynFsDirInode<DynFsKernelProviderImpl, Mutex<()>>;

    let dynfs_inode = root_inode
        .clone()
        .downcast_arc::<DynFsDirInodeImpl>()
        .map_err(|_| VfsError::Invalid)?;

    // procfs support add file manually
    dynfs_inode.add_file_manually("2", Arc::new(ProcessInfo), "r--r--r--".into())?;
    dynfs_inode.add_dir_manually("3", "r-xr-xr-x".into())?;

    println!("root dir: ");
    // readdir
    root_inode.children().for_each(|x| {
        println!("inode: {:?}", x.name);
    });

    let p = root_inode.lookup("2")?;
    let mut buf = [0; 10];
    let r = p.read_at(0, &mut buf)?;
    let content = core::str::from_utf8(&buf[..r]).unwrap();
    println!("The content is:\n{content}");

    // Procfs support remove file manually
    dynfs_inode.remove_manually("2")?;
    dynfs_inode.remove_manually("3")?;
    println!("root dir: ");
    // readdir
    root_inode.children().for_each(|x| {
        println!("inode: {:?}", x.name);
    });

    procfs.kill_sb(root_inode.get_super_block()?)?;

    Ok(())
}

struct ProcessInfo;

impl VfsFile for ProcessInfo {
    fn read_at(&self, _offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let data = b"pid:2";
        let min_len = min(data.len(), buf.len());
        buf[..min_len].copy_from_slice(&data[..min_len]);
        Ok(min_len)
    }
}

impl VfsInode for ProcessInfo {
    fn node_perm(&self) -> VfsNodePerm {
        VfsNodePerm::empty()
    }
    fn inode_type(&self) -> VfsNodeType {
        todo!()
    }
}
