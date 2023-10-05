use super::*;
use crate::inode::{basic_file_stat, RamfsInodeSame};
use crate::{KernelProvider, RamFsSuperBlock};
use alloc::sync::Arc;
use alloc::vec::Vec;
use vfscore::file::VfsFile;
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{FileStat, PollEvents, VfsNodePerm, VfsNodeType};
use vfscore::VfsResult;
pub struct RamFsFileInode<T: Send + Sync, R: VfsRawMutex> {
    basic: RamfsInodeSame<T, R>,
    inner: lock_api::Mutex<R, RamFsFileInodeInner>,
}
struct RamFsFileInodeInner {
    data: Vec<u8>,
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> RamFsFileInode<T, R> {
    pub fn new(
        sb: &Arc<RamFsSuperBlock<T, R>>,
        provider: T,
        inode_number: u64,
        perm: VfsNodePerm,
    ) -> Self {
        Self {
            basic: RamfsInodeSame::new(sb, provider, inode_number, perm),
            inner: lock_api::Mutex::new(RamFsFileInodeInner { data: Vec::new() }),
        }
    }
    pub fn update_metadata<F, Res>(&self, f: F) -> Res
    where
        F: FnOnce(&RamfsInodeSame<T, R>) -> Res,
    {
        f(&self.basic)
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsFile for RamFsFileInode<T, R> {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let inner = self.inner.lock();
        let size = inner.data.len() as u64;
        let offset = offset.min(size);
        let len = (size - offset).min(buf.len() as u64) as usize;
        let data = inner.data.as_slice();
        buf[..len].copy_from_slice(&data[offset as usize..offset as usize + len]);
        Ok(len)
    }
    fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        let mut inner = self.inner.lock();
        let buf_len = buf.len();
        let offset = offset as usize;
        let content = &mut inner.data;
        if offset + buf_len > content.len() {
            content.resize(offset + buf_len, 0);
        }
        let dst = &mut content[offset..offset + buf_len];
        dst.copy_from_slice(&buf[..dst.len()]);
        Ok(buf.len())
    }
    fn poll(&self, _event: PollEvents) -> VfsResult<PollEvents> {
        todo!()
    }
    fn ioctl(&self, _cmd: u32, _arg: u64) -> VfsResult<Option<u64>> {
        todo!()
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsInode for RamFsFileInode<T, R> {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        let res = self.basic.sb.upgrade().unwrap();
        Ok(res)
    }

    fn set_attr(&self, attr: InodeAttr) -> VfsResult<()> {
        set_attr(&self.basic, attr);
        Ok(())
    }

    fn get_attr(&self) -> VfsResult<FileStat> {
        let basic = &self.basic;
        let mut stat = basic_file_stat(basic);
        stat.st_size = self.inner.lock().data.len() as u64;
        Ok(stat)
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
        VfsNodeType::File
    }
}
