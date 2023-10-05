#![feature(trait_alias)]
#![cfg_attr(not(test), no_std)]

mod dentry;
mod inode;

extern crate alloc;

use crate::dentry::UniFsDentry;
use alloc::sync::Arc;
use vfscore::dentry::VfsDentry;
use vfscore::fstype::{FileSystemFlags, MountFlags, VfsFsType};
use vfscore::superblock::VfsSuperBlock;
use vfscore::VfsResult;

pub trait VfsRawMutex = lock_api::RawMutex + Send + Sync;

pub struct UniFs {
    real_fs: &'static str,
}

impl UniFs {
    pub fn new(name: &'static str) -> Self {
        Self { real_fs: name }
    }
}

impl VfsFsType for UniFs {
    fn mount(
        self: Arc<Self>,
        _flags: MountFlags,
        _dev_name: &str,
        _data: &[u8],
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        Ok(Arc::new(UniFsDentry))
    }

    fn kill_sb(&self, _sb: Arc<dyn VfsSuperBlock>) -> VfsResult<()> {
        Ok(())
    }
    fn fs_flag(&self) -> FileSystemFlags {
        FileSystemFlags::empty()
    }
    fn fs_name(&self) -> &'static str {
        self.real_fs
    }
}
