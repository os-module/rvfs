#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod dir;
mod file;

pub use dir::DynFsDirInode;

use alloc::sync::Arc;
use unifs::dentry::UniFsDentry;
use unifs::inode::{UniFsInodeAttr, UniFsInodeSame};
use unifs::{UniFs, UniFsSuperBlock, VfsRawMutex};
use vfscore::dentry::VfsDentry;
use vfscore::fstype::{FileSystemFlags, MountFlags, VfsFsType};
use vfscore::inode::VfsInode;
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{VfsNodePerm, VfsTimeSpec};
use vfscore::VfsResult;

pub trait DynFsKernelProvider: Send + Sync + Clone {
    fn current_time(&self) -> VfsTimeSpec;
}

pub struct DynFs<T: Send + Sync, R: VfsRawMutex>(UniFs<T, R>);

impl<T: DynFsKernelProvider + 'static, R: VfsRawMutex + 'static> DynFs<T, R> {
    pub fn new(provider: T, fs_name: &'static str) -> Self {
        Self(UniFs::new(fs_name, provider))
    }
}

impl<T: DynFsKernelProvider + 'static, R: VfsRawMutex + 'static> VfsFsType for DynFs<T, R> {
    fn mount(
        self: Arc<Self>,
        _flags: MountFlags,
        _dev_name: &str,
        _data: &[u8],
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        if self.0.sb.lock().is_none() {
            let sb = UniFsSuperBlock::new(&(self.clone() as Arc<dyn VfsFsType>));
            let root = DynFsDirInode::new(
                0,
                self.0.provider.clone(),
                &sb,
                VfsNodePerm::from_bits_truncate(0o755),
            );
            let root = Arc::new(UniFsDentry::<R>::root(Arc::new(root)));
            *sb.root.lock() = Some(root.clone());
            sb.inode_index
                .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
            sb.inode_count
                .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
            self.0.sb.lock().replace(sb.clone());
            Ok(root.clone())
        } else {
            self.0.sb.lock().as_ref().unwrap().root_dentry()
        }
    }

    fn kill_sb(&self, sb: Arc<dyn VfsSuperBlock>) -> VfsResult<()> {
        self.0.kill_sb(sb)
    }

    fn fs_flag(&self) -> FileSystemFlags {
        self.0.fs_flag()
    }

    fn fs_name(&self) -> &'static str {
        self.0.fs_name()
    }
}

trait UniInodeSameNew<T: Send + Sync, R: VfsRawMutex> {
    fn new(sb: &Arc<UniFsSuperBlock<R>>, provider: T, inode_number: u64, perm: VfsNodePerm)
        -> Self;
}

impl<T: DynFsKernelProvider + 'static, R: VfsRawMutex + 'static> UniInodeSameNew<T, R>
    for UniFsInodeSame<T, R>
{
    fn new(
        sb: &Arc<UniFsSuperBlock<R>>,
        provider: T,
        inode_number: u64,
        perm: VfsNodePerm,
    ) -> Self {
        let time = provider.current_time();
        Self {
            sb: Arc::downgrade(sb),
            inode_number,
            provider,
            inner: lock_api::Mutex::new(UniFsInodeAttr {
                link_count: 1,
                atime: time,
                mtime: time,
                ctime: time,
                perm,
            }),
        }
    }
}
