mod dir;
mod file;
mod symlink;

use alloc::collections::BTreeMap;
use alloc::string::String;
pub use dir::RamFsDirInode;
pub use file::RamFsFileInode;
pub use symlink::RamFsSymLinkInode;

use super::VfsRawMutex;
use crate::{KernelProvider, RamFsSuperBlock};
use alloc::sync::{Arc, Weak};
use vfscore::inode::InodeAttr;
use vfscore::utils::{FileStat, VfsNodePerm, VfsTimeSpec};
pub struct RamfsInodeSame<T: Send + Sync, R: VfsRawMutex> {
    pub sb: Weak<RamFsSuperBlock<T, R>>,
    pub inode_number: u64,
    pub provider: T,
    pub inner: lock_api::Mutex<R, RamFsInodeAttr>,
}
pub struct RamFsInodeAttr {
    pub link_count: u32,
    pub atime: VfsTimeSpec,
    pub mtime: VfsTimeSpec,
    pub ctime: VfsTimeSpec,
    pub perm: VfsNodePerm,
    pub ext_attr: BTreeMap<String, String>,
}
impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> RamfsInodeSame<T, R> {
    pub fn new(
        sb: &Arc<RamFsSuperBlock<T, R>>,
        provider: T,
        inode_number: u64,
        perm: VfsNodePerm,
    ) -> Self {
        let time = provider.current_time();
        Self {
            sb: Arc::downgrade(sb),
            inode_number,
            provider,
            inner: lock_api::Mutex::new(RamFsInodeAttr {
                link_count: 1,
                atime: time,
                mtime: time,
                ctime: time,
                perm,
                ext_attr: BTreeMap::new(),
            }),
        }
    }
}

fn basic_file_stat<T: Send + Sync, R: VfsRawMutex>(basic: &RamfsInodeSame<T, R>) -> FileStat {
    FileStat {
        st_dev: 0,
        st_ino: basic.inode_number,
        st_mode: 0,
        st_nlink: basic.inner.lock().link_count,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        __pad: 0,
        st_size: 0,
        st_blksize: 4096,
        __pad2: 0,
        st_blocks: 0,
        st_atime: basic.inner.lock().atime,
        st_mtime: basic.inner.lock().mtime,
        st_ctime: basic.inner.lock().ctime,
        unused: 0,
    }
}

fn set_attr<T: Send + Sync, R: VfsRawMutex>(basic: &RamfsInodeSame<T, R>, attr: InodeAttr) {
    let mut inner = basic.inner.lock();
    inner.atime = attr.atime;
    inner.mtime = attr.mtime;
    inner.ctime = attr.ctime;
}
