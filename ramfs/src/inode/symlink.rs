use super::*;
use crate::inode::{basic_file_stat, RamfsInodeSame};
use crate::{KernelProvider, RamFsSuperBlock};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use vfscore::file::VfsFile;
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{FileStat, VfsNodePerm, VfsNodeType};
use vfscore::VfsResult;
pub struct RamFsSymLinkInode<T: Send + Sync, R: VfsRawMutex> {
    basic: RamfsInodeSame<T, R>,
    inner: lock_api::Mutex<R, String>,
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> RamFsSymLinkInode<T, R> {
    pub fn new(
        sb: &Arc<RamFsSuperBlock<T, R>>,
        provider: T,
        inode_number: u64,
        sy_name: String,
    ) -> Self {
        Self {
            basic: RamfsInodeSame::new(
                sb,
                provider,
                inode_number,
                VfsNodePerm::from_bits_truncate(0o777),
            ),
            inner: lock_api::Mutex::new(sy_name),
        }
    }
    pub fn update_metadata<F, Res>(&self, f: F) -> Res
    where
        F: FnOnce(&RamfsInodeSame<T, R>) -> Res,
    {
        f(&self.basic)
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsFile for RamFsSymLinkInode<T, R> {}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsInode for RamFsSymLinkInode<T, R> {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        let res = self.basic.sb.upgrade().unwrap();
        Ok(res)
    }

    fn readlink(&self, buf: &mut [u8]) -> VfsResult<usize> {
        let inner = self.inner.lock();
        let len = inner.as_bytes().len();
        let min_len = buf.len().min(len);
        buf[..min_len].copy_from_slice(&inner.as_bytes()[..min_len]);
        Ok(min_len)
    }

    fn set_attr(&self, attr: InodeAttr) -> VfsResult<()> {
        set_attr(&self.basic, attr);
        Ok(())
    }

    fn get_attr(&self) -> VfsResult<FileStat> {
        let mut basic = basic_file_stat(&self.basic);
        basic.st_size = self.inner.lock().as_bytes().len() as u64;
        Ok(basic)
    }
    fn list_xattr(&self) -> VfsResult<Vec<String>> {
        let res = self
            .basic
            .inner
            .lock()
            .ext_attr
            .keys()
            .map(|k| k.clone())
            .collect();
        Ok(res)
    }
    fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::SymLink
    }
}
