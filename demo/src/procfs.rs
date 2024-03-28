use std::{cmp::min, error::Error, sync::Arc};

use dynfs::{DynFsDirInode, DynFsKernelProvider};
use log::info;
use spin::Mutex;
use vfscore::{
    dentry::VfsDentry, error::VfsError, file::VfsFile, fstype::VfsFsType, inode::VfsInode,
    utils::*, VfsResult,
};
pub struct ProcessInfo {
    pid: u64,
}

impl ProcessInfo {
    pub fn new(pid: u64) -> Self {
        Self { pid }
    }
}

impl VfsFile for ProcessInfo {
    fn read_at(&self, _offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let data = format!("pid:{}", self.pid);
        let data = data.as_bytes();
        let min_len = min(data.len(), buf.len());
        buf[..min_len].copy_from_slice(&data[..min_len]);
        Ok(min_len)
    }
}

impl VfsInode for ProcessInfo {
    fn node_perm(&self) -> VfsNodePerm {
        VfsNodePerm::empty()
    }
    fn get_attr(&self) -> VfsResult<VfsFileStat> {
        Ok(VfsFileStat {
            st_size: 0,
            ..Default::default()
        })
    }
    fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::File
    }
}

struct MemInfo;

impl VfsFile for MemInfo {
    fn read_at(&self, _offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let data = b"total: 1000MB\nfree: 100MB\n";
        let min_len = min(data.len(), buf.len());
        buf[..min_len].copy_from_slice(&data[..min_len]);
        Ok(min_len)
    }
}

impl VfsInode for MemInfo {
    fn node_perm(&self) -> VfsNodePerm {
        VfsNodePerm::empty()
    }
    fn get_attr(&self) -> VfsResult<VfsFileStat> {
        Ok(VfsFileStat {
            st_size: 0,
            ..Default::default()
        })
    }
    fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::File
    }
}
pub type ProcFsDirInodeImpl = DynFsDirInode<DynFsKernelProviderImpl, Mutex<()>>;

pub fn init_procfs(procfs: Arc<dyn VfsFsType>) -> Result<Arc<dyn VfsDentry>, Box<dyn Error>> {
    let root_dt = procfs.i_mount(0, "/proc", None, &[])?;

    let root_inode = root_dt.inode()?;
    let root_inode = root_inode
        .downcast_arc::<ProcFsDirInodeImpl>()
        .map_err(|_| VfsError::Invalid)?;
    let p2 = root_inode.add_dir_manually("2", "r-xr-xr-x".into())?;
    let p3 = root_inode.add_dir_manually("3", "r-xr-xr-x".into())?;
    let p2_dt = root_dt.i_insert("2", p2.clone())?;
    let p3_dt = root_dt.i_insert("3", p3.clone())?;
    let p2 = p2
        .downcast_arc::<ProcFsDirInodeImpl>()
        .map_err(|_| VfsError::Invalid)?;
    let pp2 = p2.add_file_manually("pid", Arc::new(ProcessInfo::new(2)), "r--r--r--".into())?;
    p2_dt.insert("pid", pp2)?;
    let p3 = p3
        .downcast_arc::<ProcFsDirInodeImpl>()
        .map_err(|_| VfsError::Invalid)?;
    let pp3 = p3.add_file_manually("pid", Arc::new(ProcessInfo::new(3)), "r--r--r--".into())?;
    p3_dt.insert("pid", pp3)?;
    let mem = root_inode.add_file_manually("meminfo", Arc::new(MemInfo), "r--r--r--".into())?;
    root_dt.i_insert("meminfo", mem)?;

    info!("procfs init success");
    info!("procfs tree:");
    info!(
        r"
    /
    ├── 2
        |── pid
    ├── 3
        |── pid
    |── meminfo"
    );
    Ok(root_dt)
}

#[derive(Clone)]
pub struct DynFsKernelProviderImpl;

impl DynFsKernelProvider for DynFsKernelProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}
