use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::cmp::max;

use lock_api::Mutex;
use log::trace;
use lwext4_rs::{FileTimes, MetaDataExt, Time};
use vfscore::{
    error::VfsError,
    file::VfsFile,
    impl_common_inode_default,
    inode::{InodeAttr, VfsInode},
    superblock::VfsSuperBlock,
    utils::{VfsFileStat, VfsNodePerm, VfsNodeType, VfsRenameFlag, VfsTime, VfsTimeSpec},
    VfsResult,
};

use crate::{inode::ExtFsInodeAttr, types::into_vfs, ExtFsSuperBlock, VfsRawMutex};

pub struct ExtLinkInode<R: VfsRawMutex> {
    path: String,
    sb: Weak<ExtFsSuperBlock<R>>,
    times: Mutex<R, ExtFsInodeAttr>,
}

unsafe impl<R: VfsRawMutex> Send for ExtLinkInode<R> {}
unsafe impl<R: VfsRawMutex> Sync for ExtLinkInode<R> {}

impl<R: VfsRawMutex> ExtLinkInode<R> {
    pub fn new(path: String, sb: &Arc<ExtFsSuperBlock<R>>) -> Self {
        Self {
            path,
            sb: Arc::downgrade(sb),
            times: Mutex::new(ExtFsInodeAttr::default()),
        }
    }
    pub(super) fn path(&self) -> String {
        self.path.clone()
    }
}

impl<R: VfsRawMutex + 'static> VfsFile for ExtLinkInode<R> {
    fn ioctl(&self, _cmd: u32, _arg: usize) -> VfsResult<usize> {
        Err(VfsError::NoTTY)
    }
}

impl<R: VfsRawMutex + 'static> VfsInode for ExtLinkInode<R> {
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
    fn readlink(&self, buf: &mut [u8]) -> VfsResult<usize> {
        let sb = self
            .get_super_block()?
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)?;
        trace!("[readlink] path: {:?}", self.path);
        let link = sb.fs.read_link(self.path.as_str()).map_err(into_vfs)?;
        let len = max(link.len(), buf.len());
        buf[..len].copy_from_slice(&link.as_bytes()[..len]);
        Ok(len)
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
        let times = self.times.lock();
        Ok(VfsFileStat {
            st_dev: 0,
            st_ino: meta.ino(),
            st_mode: meta.mode(),
            st_nlink: meta.nlink() as u32,
            st_uid: meta.uid(),
            st_gid: meta.gid(),
            st_rdev: 0,
            __pad: 0,
            st_size: meta.size(),
            st_blksize,
            __pad2: 0,
            st_blocks: meta.blocks(),
            // st_atime: VfsTimeSpec::new(meta.atime() as u64, 0),
            // st_mtime: VfsTimeSpec::new(meta.mtime() as u64, 0),
            // st_ctime: VfsTimeSpec::new(meta.ctime() as u64, 0),
            st_atime: times.atime,
            st_mtime: times.mtime,
            st_ctime: times.ctime,
            unused: 0,
        })
    }
    fn list_xattr(&self) -> VfsResult<Vec<String>> {
        unimplemented!()
    }
    fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::SymLink
    }
    fn update_time(&self, time: VfsTime, now: VfsTimeSpec) -> VfsResult<()> {
        let sb = self
            .get_super_block()?
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)?;
        let times = FileTimes::new();
        let mut attr_times = self.times.lock();
        match time {
            VfsTime::AccessTime(t) => {
                attr_times.atime = t;
                times.set_accessed(Time::from_extra(t.sec as u32, Some(t.nsec as u32)));
            }
            VfsTime::ModifiedTime(t) => {
                attr_times.mtime = t;
                times.set_modified(Time::from_extra(t.sec as u32, Some(t.nsec as u32)));
            }
        }
        // times.set_modified(Time::from_extra(now.sec as u32, Some(now.nsec as u32)));
        trace!("[update_time] path: {:?}, times: {:?}", self.path, times);
        attr_times.ctime = now;
        sb.fs.set_times(&self.path, times).map_err(into_vfs)
    }
}
