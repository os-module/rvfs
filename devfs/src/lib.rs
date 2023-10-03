#![feature(trait_alias)]
#![cfg_attr(not(test), no_std)]
extern crate alloc;
use alloc::sync::Arc;
use vfscore::dentry::VfsDentry;
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::fstype::{FileSystemFlags, VfsFsType};
use vfscore::VfsResult;

pub trait VfsRawMutex = lock_api::RawMutex + Send + Sync;
#[derive(Default, Debug)]
pub struct DevFs;

impl DevFs {
    pub fn new() -> Self {
        Self::default()
    }
}

impl VfsFsType for DevFs {
    fn get_fs_flag(&self) -> FileSystemFlags {
        FileSystemFlags::empty()
    }

    fn fs_name(&self) -> &'static str {
        "devfs"
    }

    fn make_vfs_file(&self, _dentry: Arc<dyn VfsDentry>) -> VfsResult<Arc<dyn VfsFile>> {
        Err(VfsError::NoSys)
    }
}
