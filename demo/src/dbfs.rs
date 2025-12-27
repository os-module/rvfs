use std::sync::Arc;
use vfscore::{dentry::VfsDentry, fstype::VfsFsType, utils::VfsTimeSpec};
use dbfs_vfs::DBFSProvider;

#[derive(Clone)]
pub struct DBFSProviderImpl;

impl DBFSProvider for DBFSProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}

pub fn init_dbfs(dbfs: Arc<dyn VfsFsType>) -> Result<Arc<dyn VfsDentry>, Box<dyn std::error::Error>> {
    let root_dt = dbfs.i_mount(0, "/", None, &[])?;
    Ok(root_dt)
}
