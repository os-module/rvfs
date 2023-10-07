use crate::{DevInodeSameNew, DevKernelProvider, UniFsSuperBlock};
use alloc::sync::Arc;
use unifs::inode::{basic_file_stat, UniFsInodeSame};
use unifs::*;
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{FileStat, PollEvents, VfsNodePerm, VfsNodeType};
use vfscore::VfsResult;

pub struct DevFsDevInode<T: Send + Sync, R: VfsRawMutex> {
    rdev: u32,
    basic: UniFsInodeSame<T, R>,
    ty: VfsNodeType,
}

impl<T: DevKernelProvider + 'static, R: VfsRawMutex + 'static> DevFsDevInode<T, R> {
    pub fn new(
        sb: &Arc<UniFsSuperBlock<R>>,
        provider: T,
        inode_number: u64,
        rdev: u32,
        ty: VfsNodeType,
    ) -> Self {
        Self {
            rdev,
            basic: UniFsInodeSame::new(
                sb,
                provider,
                inode_number,
                VfsNodePerm::from_bits_truncate(0o666),
            ),
            ty,
        }
    }

    pub fn real_dev(&self) -> VfsResult<Arc<dyn VfsInode>> {
        let dev = self.basic.provider.rdev2device(self.rdev);
        if dev.is_none() {
            return Err(VfsError::NoDev);
        }
        Ok(dev.unwrap())
    }
}

impl<T: DevKernelProvider + 'static, R: VfsRawMutex + 'static> VfsFile for DevFsDevInode<T, R> {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        self.real_dev()?.read_at(offset, buf)
    }
    fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        self.real_dev()?.write_at(offset, buf)
    }
    fn poll(&self, event: PollEvents) -> VfsResult<PollEvents> {
        self.real_dev()?.poll(event)
    }

    fn ioctl(&self, cmd: u32, arg: u64) -> VfsResult<Option<u64>> {
        self.real_dev()?.ioctl(cmd, arg)
    }
    fn flush(&self) -> VfsResult<()> {
        self.real_dev()?.flush()
    }

    fn fsync(&self) -> VfsResult<()> {
        self.real_dev()?.fsync()
    }
}

impl<T: DevKernelProvider + 'static, R: VfsRawMutex + 'static> VfsInode for DevFsDevInode<T, R> {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        let res = self.basic.sb.upgrade().ok_or(VfsError::Invalid);
        res.map(|sb| sb as Arc<dyn VfsSuperBlock>)
    }

    fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        todo!()
    }

    fn get_attr(&self) -> VfsResult<FileStat> {
        let attr = basic_file_stat(&self.basic);
        Ok(attr)
    }

    fn inode_type(&self) -> VfsNodeType {
        self.ty
    }
}
