#![cfg_attr(not(test), no_std)]
#![feature(trait_alias)]
extern crate alloc;

mod inode;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
pub use inode::*;
use log::info;
use unifs::dentry::UniFsDentry;
use unifs::*;
use vfscore::dentry::VfsDentry;
use vfscore::error::VfsError;
use vfscore::fstype::{FileSystemFlags, VfsFsType};
use vfscore::inode::VfsInode;
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{VfsNodePerm, VfsTimeSpec};
use vfscore::VfsResult;

pub trait KernelProvider: Send + Sync + Clone {
    fn current_time(&self) -> VfsTimeSpec;
}

pub struct RamFs<T: Send + Sync, R: VfsRawMutex> {
    provider: T,
    fs_container: lock_api::Mutex<R, BTreeMap<usize, UniFs<T, R>>>,
}

impl<T: KernelProvider, R: VfsRawMutex + 'static> RamFs<T, R> {
    pub fn new(provider: T) -> Self {
        Self {
            provider,
            fs_container: lock_api::Mutex::new(BTreeMap::new()),
        }
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsFsType for RamFs<T, R> {
    fn mount(
        self: Arc<Self>,
        _flags: u32,
        _dev: Option<Arc<dyn VfsInode>>,
        _data: &[u8],
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        let unifs = UniFs::<T, R>::new("ramfs", self.provider.clone());
        let sb = UniFsSuperBlock::new(&(self.clone() as Arc<dyn VfsFsType>));
        let root = Arc::new(RamFsDirInode::new(
            &sb,
            self.provider.clone(),
            0,
            VfsNodePerm::from_bits_truncate(0o755),
        ));
        let root_dentry = Arc::new(UniFsDentry::<R>::root(root));
        sb.inode_index
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        sb.inode_count
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        sb.root.lock().replace(root_dentry.clone());
        let sb_ptr = Arc::as_ptr(&sb) as usize;
        unifs.sb.lock().replace(sb);

        self.fs_container.lock().insert(sb_ptr, unifs);
        Ok(root_dentry)
    }

    fn kill_sb(&self, sb: Arc<dyn VfsSuperBlock>) -> VfsResult<()> {
        let sb = sb
            .downcast_arc::<UniFsSuperBlock<R>>()
            .map_err(|_| VfsError::Invalid)?;
        let sb_ptr = Arc::as_ptr(&sb) as usize;
        if self.fs_container.lock().remove(&sb_ptr).is_some() {
            info!("kill ramfs sb success");
            Ok(())
        } else {
            Err(VfsError::Invalid)
        }
    }
    fn fs_flag(&self) -> FileSystemFlags {
        FileSystemFlags::empty()
    }

    fn fs_name(&self) -> &'static str {
        "ramfs"
    }
}
