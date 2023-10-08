use crate::fs::FatFsSuperBlock;
use crate::FatDir;
use crate::*;
use alloc::sync::Weak;
use vfscore::utils::VfsNodePerm;

pub struct FatFsDirInode<R: VfsRawMutex> {
    dir: Mutex<R, FatDir>,
    attr: FatFsInodeSame<R>,
}

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

impl<R: VfsRawMutex> FatFsDirInode<R> {}
