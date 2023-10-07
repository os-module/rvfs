#![feature(trait_alias)]
#![cfg_attr(not(test), no_std)]

pub mod dentry;
pub mod inode;

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::{Arc, Weak};
use core::sync::atomic::{AtomicU64, AtomicUsize};
use log::info;
use vfscore::dentry::VfsDentry;
use vfscore::error::VfsError;
use vfscore::fstype::{FileSystemFlags, VfsFsType};
use vfscore::inode::VfsInode;
use vfscore::superblock::{SuperType, VfsSuperBlock};
use vfscore::utils::{VfsFsStat, VfsTimeSpec};
use vfscore::VfsResult;

pub trait VfsRawMutex = lock_api::RawMutex + Send + Sync;
pub struct UniFs<T: Send + Sync, R: VfsRawMutex> {
    real_fs: &'static str,
    pub provider: T,
    pub sb: lock_api::Mutex<R, Option<Arc<UniFsSuperBlock<R>>>>,
}

impl<T: Send + Sync, R: VfsRawMutex + 'static> UniFs<T, R> {
    pub fn new(name: &'static str, provider: T) -> Self {
        Self {
            real_fs: name,
            provider,
            sb: lock_api::Mutex::new(None),
        }
    }
}

impl<T: Send + Sync, R: VfsRawMutex + 'static> UniFs<T, R> {
    pub fn kill_sb(&self, sb: Arc<dyn VfsSuperBlock>) -> VfsResult<()> {
        let t_sb = sb
            .downcast_arc::<UniFsSuperBlock<R>>()
            .map_err(|_| VfsError::Invalid)?;
        let mut sb = self.sb.lock();
        if sb.is_none() {
            return Err(VfsError::Invalid);
        }
        let old_sb = sb.as_ref().unwrap();
        if !Arc::ptr_eq(old_sb, &t_sb) {
            return Err(VfsError::Invalid);
        }
        *sb = None;
        info!("{} killed", self.real_fs);
        Ok(())
    }
    pub fn fs_flag(&self) -> FileSystemFlags {
        FileSystemFlags::empty()
    }
    pub fn fs_name(&self) -> &'static str {
        self.real_fs
    }
}

pub struct UniFsSuperBlock<R: VfsRawMutex> {
    fs_type: Weak<dyn VfsFsType>,
    pub root: lock_api::Mutex<R, Option<Arc<dyn VfsDentry>>>,
    pub inode_index: AtomicU64,
    pub inode_count: AtomicUsize,
    inode_cache: lock_api::Mutex<R, BTreeMap<u64, Arc<dyn VfsInode>>>,
}

impl<R: VfsRawMutex + 'static> UniFsSuperBlock<R> {
    /// Call this function only once
    pub fn new(fs_type: &Arc<dyn VfsFsType>) -> Arc<Self> {
        Arc::new(Self {
            fs_type: Arc::downgrade(fs_type),
            root: lock_api::Mutex::new(None),
            inode_index: AtomicU64::new(0),
            inode_count: AtomicUsize::new(0),
            inode_cache: lock_api::Mutex::new(BTreeMap::new()),
        })
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
        cache.get(&inode_number).cloned()
    }
}

impl<R: VfsRawMutex + 'static> VfsSuperBlock for UniFsSuperBlock<R> {
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
        SuperType::Single
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
