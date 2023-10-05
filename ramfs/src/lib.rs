#![cfg_attr(not(test), no_std)]
#![feature(trait_alias)]
extern crate alloc;
mod dentry;
mod inode;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize};
pub use dentry::RamFsDentry;
pub use inode::*;
use lock_api::RawMutex;
use log::info;
use vfscore::dentry::VfsDentry;
use vfscore::error::VfsError;
use vfscore::fstype::{FileSystemFlags, MountFlags, VfsFsType};
use vfscore::inode::VfsInode;
use vfscore::superblock::{SuperType, VfsSuperBlock};
use vfscore::utils::{VfsFsStat, VfsTimeSpec};
use vfscore::VfsResult;
pub trait VfsRawMutex = RawMutex + Send + Sync;
pub trait KernelProvider: Send + Sync + Clone {
    fn current_time(&self) -> VfsTimeSpec;
}

pub struct RamFs<T: Send + Sync, R: VfsRawMutex> {
    provider: T,
    sbs: lock_api::Mutex<R, Vec<Arc<RamFsSuperBlock<T, R>>>>,
}

impl<T: KernelProvider, R: VfsRawMutex + 'static> RamFs<T, R> {
    pub fn new(provider: T) -> Self {
        Self {
            provider,
            sbs: lock_api::Mutex::new(Vec::new()),
        }
    }
}

pub struct RamFsSuperBlock<T: Send + Sync, R: VfsRawMutex> {
    fs_type: Weak<RamFs<T, R>>,
    root: lock_api::Mutex<R, Option<Arc<RamFsDentry<T, R>>>>,
    inode_index: AtomicU64,
    inode_count: AtomicUsize,
    inode_cache: lock_api::Mutex<R, BTreeMap<u64, Arc<dyn VfsInode>>>,
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> RamFsSuperBlock<T, R> {
    /// Call this function only once
    pub fn new(fs_type: &Arc<RamFs<T, R>>, provider: T) -> Arc<Self> {
        let sb = Arc::new(Self {
            fs_type: Arc::downgrade(fs_type),
            root: lock_api::Mutex::new(None),
            inode_index: AtomicU64::new(0),
            inode_count: AtomicUsize::new(0),
            inode_cache: lock_api::Mutex::new(BTreeMap::new()),
        });
        let root = Arc::new(RamFsDentry::root(provider, &sb));
        *sb.root.lock() = Some(root.clone());
        sb
    }
    pub fn insert_inode(&self, inode_number: u64, inode: Arc<dyn VfsInode>) {
        let mut cache = self.inode_cache.lock();
        cache.insert(inode_number, inode);
    }
    pub fn remove_inode(&self, inode_number: u64) {
        let mut cache = self.inode_cache.lock();
        cache.remove(&inode_number);
    }
    pub fn get_inode(&self, inode_number: u64) -> Option<Arc<dyn VfsInode>> {
        let cache = self.inode_cache.lock();
        cache.get(&inode_number).map(|inode| inode.clone())
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsSuperBlock
    for RamFsSuperBlock<T, R>
{
    fn sync_fs(&self, _wait: bool) -> VfsResult<()> {
        Ok(())
    }

    fn stat_fs(&self) -> VfsResult<VfsFsStat> {
        Ok(VfsFsStat {
            f_type: 0,
            f_bsize: 4096,
            f_blocks: 0,
            f_bfree: 0,
            f_bavail: 0,
            f_files: 0,
            f_ffree: 0,
            f_fsid: [0, 0],
            f_namelen: 0,
            f_frsize: 0,
            f_flags: 0,
            f_spare: [0; 4],
        })
    }

    fn super_type(&self) -> SuperType {
        SuperType::Independent
    }

    fn fs_type(&self) -> Arc<dyn VfsFsType> {
        self.fs_type.upgrade().unwrap()
    }

    fn root_dentry(&self) -> VfsResult<Arc<dyn VfsDentry>> {
        let lock = self.root.lock();
        if let Some(root) = &*lock {
            Ok(root.clone())
        } else {
            Err(VfsError::Invalid)
        }
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsFsType for RamFs<T, R> {
    fn mount(
        self: Arc<Self>,
        _flags: MountFlags,
        _dev_name: &str,
        _data: &[u8],
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        let sb = RamFsSuperBlock::new(&self, self.provider.clone());
        self.sbs.lock().push(sb.clone());
        sb.root_dentry()
    }

    fn kill_sb(&self, sb: Arc<dyn VfsSuperBlock>) -> VfsResult<()> {
        if let Ok(sb) = sb.downcast_arc::<RamFsSuperBlock<T, R>>() {
            let mut sbs = self.sbs.lock();
            sbs.retain(|x| !Arc::ptr_eq(x, &sb));
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
