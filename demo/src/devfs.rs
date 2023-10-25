use devfs::DevKernelProvider;
use log::info;
use std::error::Error;
use std::sync::Arc;
use vfscore::dentry::VfsDentry;
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::fstype::VfsFsType;
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{FileStat, VfsNodePerm, VfsNodeType, VfsTimeSpec};
use vfscore::VfsResult;

#[derive(Clone)]
pub struct DevFsKernelProviderImpl;

impl DevKernelProvider for DevFsKernelProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
    fn rdev2device(&self, rdev: u64) -> Option<Arc<dyn VfsInode>> {
        match rdev {
            0 => Some(Arc::new(NullDev)),
            1 => Some(Arc::new(NullDev)),
            _ => None,
        }
    }
}
struct NullDev;

impl VfsFile for NullDev {
    fn read_at(&self, _offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        buf.fill(0);
        Ok(buf.len())
    }
    fn write_at(&self, _offset: u64, buf: &[u8]) -> VfsResult<usize> {
        Ok(buf.len())
    }
}

impl VfsInode for NullDev {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        todo!()
    }

    fn node_perm(&self) -> VfsNodePerm {
        VfsNodePerm::empty()
    }

    fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        Ok(())
    }

    fn get_attr(&self) -> VfsResult<FileStat> {
        Err(VfsError::NoSys)
    }

    fn inode_type(&self) -> VfsNodeType {
        todo!()
    }
}

pub fn init_devfs(devfs: Arc<dyn VfsFsType>) -> Result<Arc<dyn VfsDentry>, Box<dyn Error>> {
    let root_dt = devfs.i_mount(0, None, &[])?;
    let root_inode = root_dt.inode()?;
    let null = root_inode.create(
        "null",
        VfsNodeType::CharDevice,
        VfsNodePerm::from_bits_truncate(0o666),
        Some(0),
    )?;
    let zero = root_inode.create(
        "zero",
        VfsNodeType::CharDevice,
        VfsNodePerm::from_bits_truncate(0o666),
        Some(1),
    )?;
    root_dt.i_insert("null", null.clone())?;
    root_dt.i_insert("zero", zero.clone())?;
    info!("devfs init success");
    info!("devfs tree:");
    info!(
        r"
    /
    ├── null
    └── zero"
    );

    Ok(root_dt)
}
