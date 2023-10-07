use dynfs::{DynFs, DynFsDirInode, DynFsKernelProvider};
use spin::Mutex;
use std::cmp::min;
use std::error::Error;
use std::sync::Arc;
use vfscore::file::VfsFile;
use vfscore::fstype::{MountFlags, VfsFsType};
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{FileStat, VfsNodeType, VfsTimeSpec};
use vfscore::VfsResult;
struct ProcessInfo;

impl VfsFile for ProcessInfo {
    fn read_at(&self, _offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let data = b"pid:2\n";
        let min_len = min(data.len(), buf.len());
        buf[..min_len].copy_from_slice(&data[..min_len]);
        Ok(min_len)
    }
}

impl VfsInode for ProcessInfo {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        todo!()
    }

    fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        todo!()
    }

    fn get_attr(&self) -> VfsResult<FileStat> {
        todo!()
    }

    fn inode_type(&self) -> VfsNodeType {
        todo!()
    }
}

pub fn init_procfs() -> Result<(), Box<dyn Error>> {
    let procfs = Arc::new(DynFs::<_, Mutex<()>>::new(
        DynFsKernelProviderImpl,
        "procfs",
    ));
    let root_dt = procfs.clone().mount(MountFlags::empty(), "", &[])?;
    let root_inode = root_dt.inode()?;
    let dynfs_inode = root_inode
        .downcast_arc::<DynFsDirInode<DynFsKernelProviderImpl, Mutex<()>>>()
        .unwrap();
    dynfs_inode.add_file_manually("2", Arc::new(ProcessInfo), "r--r--r--".into())?;
    dynfs_inode.add_dir_manually("3", "r-xr-xr-x".into())?;
    Ok(())
}

#[derive(Clone)]
struct DynFsKernelProviderImpl;

impl DynFsKernelProvider for DynFsKernelProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}
