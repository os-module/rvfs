use crate::types::into_vfs;
use crate::{ExtFsSuperBlock, VfsRawMutex};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use embedded_io::{Read, Seek, SeekFrom, Write};
use lock_api::Mutex;
use log::info;
use lwext4_rs::{File, FileTimes, MetaDataExt, Time};
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{VfsFileStat, VfsNodePerm, VfsNodeType, VfsRenameFlag, VfsTime, VfsTimeSpec};
use vfscore::{impl_file_inode_default, VfsResult};
pub struct ExtFileInode<R: VfsRawMutex> {
    file: Mutex<R, File>,
    sb: Weak<ExtFsSuperBlock<R>>,
}

unsafe impl<R: VfsRawMutex> Send for ExtFileInode<R> {}
unsafe impl<R: VfsRawMutex> Sync for ExtFileInode<R> {}

impl<R: VfsRawMutex> ExtFileInode<R> {
    pub fn new(file: File, sb: &Arc<ExtFsSuperBlock<R>>) -> Self {
        Self {
            file: Mutex::new(file),
            sb: Arc::downgrade(sb),
        }
    }
    pub(super) fn path(&self) -> String {
        self.file.lock().path()
    }
}

impl<R: VfsRawMutex + 'static> VfsFile for ExtFileInode<R> {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let mut file = self.file.lock();
        if file.stream_position().map_err(into_vfs)? != offset {
            file.seek(SeekFrom::Start(offset)).map_err(into_vfs)?;
        }
        file.read(buf).map_err(into_vfs)
    }
    fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        let mut file = self.file.lock();
        if file.stream_position().map_err(into_vfs)? != offset {
            file.seek(SeekFrom::Start(offset)).map_err(into_vfs)?;
        }
        file.write(buf).map_err(into_vfs)
    }
    fn ioctl(&self, _cmd: u32, _arg: usize) -> VfsResult<usize> {
        Err(VfsError::NoTTY)
    }
    fn flush(&self) -> VfsResult<()> {
        self.fsync()
    }
    fn fsync(&self) -> VfsResult<()> {
        self.file.lock().flush().map_err(into_vfs)
    }
}

impl<R: VfsRawMutex + 'static> VfsInode for ExtFileInode<R> {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        Ok(self.sb.upgrade().unwrap())
    }
    impl_file_inode_default!();
    fn node_perm(&self) -> VfsNodePerm {
        let file = self.file.lock();
        let perm = file.metadata().map_or(VfsNodePerm::default_dir(), |meta| {
            VfsNodePerm::from_bits_truncate(meta.permissions().mode() as u16)
        });
        perm
    }
    fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        Ok(())
    }
    fn get_attr(&self) -> VfsResult<VfsFileStat> {
        let file = self.file.lock();
        let meta = file.metadata().map_err(into_vfs)?;
        let sb = self
            .get_super_block()?
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)?;
        let fs_stat = sb.fs.mount_handle().stats().map_err(into_vfs)?;
        let st_blksize = fs_stat.block_size;
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
        VfsNodeType::File
    }
    fn truncate(&self, len: u64) -> VfsResult<()> {
        self.file.lock().set_len(len).map_err(into_vfs)
    }
    fn update_time(&self, time: VfsTime, now: VfsTimeSpec) -> VfsResult<()> {
        let mut file = self.file.lock();
        let mut times = FileTimes::new();
        match time {
            VfsTime::AccessTime(t) => {
                times = times.set_accessed(Time::from_extra(t.sec as u32, Some(t.nsec as u32)));
            }
            VfsTime::ModifiedTime(t) => {
                times = times.set_modified(Time::from_extra(t.sec as u32, Some(t.nsec as u32)));
            }
        }
        times = times.set_modified(Time::from_extra(now.sec as u32, Some(now.nsec as u32)));
        info!("[update_time] path: {:?}, times: {:?}", file.path(), times);
        file.set_times(times).map_err(into_vfs)
    }
}
