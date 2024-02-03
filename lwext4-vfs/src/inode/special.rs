use crate::types::into_vfs;
use crate::{ExtFsSuperBlock, VfsRawMutex};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use lwext4_rs::{FileTimes, MetaDataExt, Time};
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{
    VfsFileStat, VfsNodePerm, VfsNodeType, VfsPollEvents, VfsRenameFlag, VfsTime, VfsTimeSpec,
};
use vfscore::{impl_common_inode_default, VfsResult};

pub trait ExtDevProvider: Send + Sync {
    fn rdev2device(&self, rdev: u64) -> Option<Arc<dyn VfsInode>>;
}

pub struct ExtSpecialInode<R: VfsRawMutex> {
    path: String,
    sb: Weak<ExtFsSuperBlock<R>>,
    ty: VfsNodeType,
    rdev: u64,
    provider: Arc<dyn ExtDevProvider>,
}

unsafe impl<R: VfsRawMutex> Send for ExtSpecialInode<R> {}
unsafe impl<R: VfsRawMutex> Sync for ExtSpecialInode<R> {}

impl<R: VfsRawMutex> ExtSpecialInode<R> {
    pub fn new(
        path: String,
        sb: &Arc<ExtFsSuperBlock<R>>,
        ty: VfsNodeType,
        rdev: u64,
        provider: Arc<dyn ExtDevProvider>,
    ) -> Self {
        Self {
            path,
            sb: Arc::downgrade(sb),
            ty,
            rdev,
            provider,
        }
    }
    pub(super) fn real_dev(&self) -> VfsResult<Arc<dyn VfsInode>> {
        let dev = self.provider.rdev2device(self.rdev);
        if dev.is_none() {
            return Err(VfsError::NoDev);
        }
        Ok(dev.unwrap())
    }

    pub(super) fn path(&self) -> String {
        self.path.clone()
    }
}

impl<R: VfsRawMutex + 'static> VfsFile for ExtSpecialInode<R> {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        self.real_dev()?.read_at(offset, buf)
    }
    fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        self.real_dev()?.write_at(offset, buf)
    }
    fn poll(&self, event: VfsPollEvents) -> VfsResult<VfsPollEvents> {
        self.real_dev()?.poll(event)
    }

    fn ioctl(&self, _cmd: u32, _arg: usize) -> VfsResult<usize> {
        self.real_dev()?.ioctl(_cmd, _arg)
    }
    fn flush(&self) -> VfsResult<()> {
        self.real_dev()?.flush()
    }

    fn fsync(&self) -> VfsResult<()> {
        self.real_dev()?.fsync()
    }
}

impl<R: VfsRawMutex + 'static> VfsInode for ExtSpecialInode<R> {
    impl_common_inode_default!();

    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        Ok(self.sb.upgrade().unwrap())
    }
    fn node_perm(&self) -> VfsNodePerm {
        let sb = self
            .get_super_block()
            .unwrap()
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)
            .unwrap();
        let perm = sb
            .fs
            .metadata(self.path.as_str())
            .map_or(VfsNodePerm::default_dir(), |meta| {
                VfsNodePerm::from_bits_truncate(meta.permissions().mode() as u16)
            });
        perm
    }

    fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        Ok(())
    }
    fn get_attr(&self) -> VfsResult<VfsFileStat> {
        let sb = self
            .get_super_block()?
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)?;
        let meta = sb.fs.metadata(self.path.as_str()).map_err(into_vfs)?;
        let fs_stat = sb.fs.mount_handle().stats().map_err(into_vfs)?;
        let st_blksize = fs_stat.block_size;
        Ok(VfsFileStat {
            st_dev: 0,
            st_ino: meta.ino(),
            st_mode: meta.mode(),
            st_nlink: meta.nlink() as u32,
            st_uid: meta.uid(),
            st_gid: meta.gid(),
            st_rdev: self.rdev,
            __pad: 0,
            st_size: meta.size(),
            st_blksize,
            __pad2: 0,
            st_blocks: meta.blocks(),
            st_atime: VfsTimeSpec::new(meta.atime() as u64, 0),
            st_mtime: VfsTimeSpec::new(meta.mtime() as u64, 0),
            st_ctime: VfsTimeSpec::new(meta.ctime() as u64, 0),
            unused: 0,
        })
    }
    fn list_xattr(&self) -> VfsResult<Vec<String>> {
        unimplemented!()
    }
    fn inode_type(&self) -> VfsNodeType {
        self.ty
    }
    fn update_time(&self, time: VfsTime, now: VfsTimeSpec) -> VfsResult<()> {
        let sb = self
            .get_super_block()?
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)?;
        let times = FileTimes::new();
        match time {
            VfsTime::AccessTime(t) => {
                times.set_accessed(Time::from_extra(t.sec as u32, Some(t.nsec as u32)));
            }
            VfsTime::ModifiedTime(t) => {
                times.set_modified(Time::from_extra(t.sec as u32, Some(t.nsec as u32)));
            }
        }
        times.set_modified(Time::from_extra(now.sec as u32, Some(now.nsec as u32)));
        sb.fs.set_times(&self.path, times).map_err(into_vfs)
    }
}
