use crate::fs::FatFsSuperBlock;
use crate::inode::FatFsInodeSame;
use crate::*;
use alloc::string::String;
use alloc::sync::Weak;
use fatfs::{Read, Seek, Write};
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{FileStat, PollEvents, VfsNodePerm, VfsNodeType};
use vfscore::VfsResult;

pub struct FatFsFileInode<R: VfsRawMutex> {
    parent: Weak<Mutex<R, FatDir>>,
    file: Arc<Mutex<R, FatFile>>,
    attr: FatFsInodeSame<R>,
    name: String,
}

impl<R: VfsRawMutex + 'static> FatFsFileInode<R>
where
    R: VfsRawMutex,
{
    pub fn new(
        parent: &Arc<Mutex<R, FatDir>>,
        file: Arc<Mutex<R, FatFile>>,
        sb: &Arc<FatFsSuperBlock<R>>,
        name: String,
        perm: VfsNodePerm,
    ) -> Self {
        Self {
            name,
            parent: Arc::downgrade(parent),
            file,
            attr: FatFsInodeSame::new(sb, perm),
        }
    }
}

impl<R: VfsRawMutex + 'static> VfsFile for FatFsFileInode<R> {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let mut file = self.file.lock();
        let fat_offset = file.offset();
        if offset != fat_offset as u64 {
            file.seek(fatfs::SeekFrom::Start(offset))
                .map_err(|_| VfsError::IoError)?;
        }
        let len = file.read(buf).map_err(|_| VfsError::IoError)?;
        Ok(len)
    }
    fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        let mut file = self.file.lock();
        let fat_offset = file.offset();
        if offset != fat_offset as u64 {
            file.seek(fatfs::SeekFrom::Start(offset))
                .map_err(|_| VfsError::IoError)?;
        }
        let len = file.write(buf).map_err(|_| VfsError::IoError)?;
        Ok(len)
    }
    fn poll(&self, _event: PollEvents) -> VfsResult<PollEvents> {
        todo!()
    }
    fn flush(&self) -> VfsResult<()> {
        self.fsync()
    }
    fn fsync(&self) -> VfsResult<()> {
        self.file.lock().flush().map_err(|_| VfsError::IoError)
    }
}

impl<R: VfsRawMutex + 'static> VfsInode for FatFsFileInode<R> {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        let sb = self.attr.sb.upgrade().unwrap();
        Ok(sb)
    }

    fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        todo!()
    }

    fn get_attr(&self) -> VfsResult<FileStat> {
        let attr = self.attr.inner.lock();
        let parent = self.parent.upgrade().unwrap();
        let find = parent.lock().iter().find(|x| {
            if let Ok(d) = x {
                d.is_file() && d.file_name() == self.name
            } else {
                false
            }
        });
        if find.is_none() {
            return Err(VfsError::IoError);
        }
        let find = find.unwrap();
        if find.is_err() {
            return Err(VfsError::IoError);
        }
        let len = find.unwrap().len();
        Ok(FileStat {
            st_dev: 0,
            st_ino: 1,
            st_mode: attr.perm.bits() as u32,
            st_nlink: 1,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size: len,
            st_blksize: 512,
            __pad2: 0,
            st_blocks: len / 512,
            st_atime: attr.atime,
            st_mtime: attr.mtime,
            st_ctime: attr.ctime,
            unused: 0,
        })
    }

    fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::File
    }
}
