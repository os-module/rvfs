mod dir;
mod file;

use crate::fs::FatFsSuperBlock;
use crate::*;
use alloc::sync::Weak;
use vfscore::utils::VfsNodePerm;

pub use dir::*;
pub use file::*;

struct FatFsInodeSame<R: VfsRawMutex> {
    pub sb: Weak<FatFsSuperBlock<R>>,
    pub inner: Mutex<R, FatFsInodeAttr>,
}
struct FatFsInodeAttr {
    pub atime: VfsTimeSpec,
    pub mtime: VfsTimeSpec,
    pub ctime: VfsTimeSpec,
    pub perm: VfsNodePerm,
}

impl<R: VfsRawMutex> FatFsInodeSame<R> {
    pub fn new(sb: &Arc<FatFsSuperBlock<R>>, perm: VfsNodePerm) -> Self {
        Self {
            sb: Arc::downgrade(sb),
            inner: Mutex::new(FatFsInodeAttr {
                atime: VfsTimeSpec::new(0, 0),
                mtime: VfsTimeSpec::new(0, 0),
                ctime: VfsTimeSpec::new(0, 0),
                perm,
            }),
        }
    }
}
