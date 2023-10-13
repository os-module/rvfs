use crate::fs::FatFsSuperBlock;
use crate::inode::{FatFsFileInode, FatFsInodeSame};
use crate::*;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Weak;
use fatfs::Error;
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{FileStat, VfsDirEntry, VfsNodePerm, VfsNodeType};
use vfscore::VfsResult;

pub struct FatFsDirInode<R: VfsRawMutex> {
    #[allow(unused)]
    parent: Weak<Mutex<R, FatDir>>,
    dir: Arc<Mutex<R, FatDir>>,
    attr: FatFsInodeSame<R>,
    inode_cache: Mutex<R, BTreeMap<String, Arc<dyn VfsInode>>>,
}

impl<R: VfsRawMutex> FatFsDirInode<R> {
    pub fn new(
        parent: &Arc<Mutex<R, FatDir>>,
        dir: Arc<Mutex<R, FatDir>>,
        sb: &Arc<FatFsSuperBlock<R>>,
        perm: VfsNodePerm,
    ) -> Self {
        Self {
            parent: Arc::downgrade(parent),
            dir,
            attr: FatFsInodeSame::new(sb, perm),
            inode_cache: Mutex::new(BTreeMap::new()),
        }
    }

    fn delete_file(&self, name: &str, ty: VfsNodeType) -> VfsResult<()> {
        let mut inode_cache = self.inode_cache.lock();
        if let Some((_, inode)) = inode_cache.iter().find(|(k, _)| *k == name) {
            assert_eq!(inode.inode_type(), ty);
            inode_cache.remove(name);
        }
        let dir = self.dir.lock();
        dir.remove(name).map_err(|e| match e {
            Error::NotFound | Error::InvalidInput => VfsError::NoEntry,
            _ => VfsError::IoError,
        })?;
        Ok(())
    }
}

impl<R: VfsRawMutex + 'static> VfsFile for FatFsDirInode<R> {
    fn readdir(&self, start_index: usize) -> VfsResult<Option<VfsDirEntry>> {
        let entry = self.dir.lock().iter().nth(start_index);
        if let Some(entry) = entry {
            match entry {
                Ok(entry) => {
                    let ty = if entry.is_dir() {
                        VfsNodeType::Dir
                    } else {
                        VfsNodeType::File
                    };
                    let entry = VfsDirEntry {
                        ino: 1,
                        ty,
                        name: entry.file_name(),
                    };
                    Ok(Some(entry))
                }
                Err(_e) => Err(VfsError::IoError),
            }
        } else {
            Ok(None)
        }
    }
}

impl<R: VfsRawMutex + 'static> VfsInode for FatFsDirInode<R> {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        let sb = self.attr.sb.upgrade().unwrap();
        Ok(sb)
    }
    fn create(
        &self,
        name: &str,
        ty: VfsNodeType,
        perm: VfsNodePerm,
        _rdev: Option<u64>,
    ) -> VfsResult<Arc<dyn VfsInode>> {
        let mut inode_cache = self.inode_cache.lock();
        if inode_cache.contains_key(name) {
            return Err(VfsError::FileExist);
        }
        match ty {
            VfsNodeType::Dir => {
                let new_dir = self
                    .dir
                    .lock()
                    .create_dir(name)
                    .map_err(|_| VfsError::IoError)?;
                let new_dir = Arc::new(Mutex::new(new_dir));

                let inode =
                    FatFsDirInode::new(&self.dir, new_dir, &self.attr.sb.upgrade().unwrap(), perm);
                let inode = Arc::new(inode);
                inode_cache.insert(name.to_string(), inode.clone());
                Ok(inode)
            }
            VfsNodeType::File => {
                let file = self
                    .dir
                    .lock()
                    .create_file(name)
                    .map_err(|_| VfsError::IoError)?;
                let file = Arc::new(Mutex::new(file));
                let inode = FatFsFileInode::new(
                    &self.dir,
                    file,
                    &self.attr.sb.upgrade().unwrap(),
                    name.to_string(),
                    perm,
                );
                let inode = Arc::new(inode);
                inode_cache.insert(name.to_string(), inode.clone());
                Ok(inode)
            }
            _ => Err(VfsError::Invalid),
        }
    }

    fn unlink(&self, name: &str) -> VfsResult<()> {
        self.delete_file(name, VfsNodeType::File)
    }

    fn lookup(&self, name: &str) -> VfsResult<Option<Arc<dyn VfsInode>>> {
        let mut inode_cache = self.inode_cache.lock();
        if let Some(inode) = inode_cache.get(name) {
            return Ok(Some(inode.clone()));
        }
        let dir = self.dir.lock();
        let new_dir = dir
            .open_dir(name)
            .map_err(|e| !matches!(e, Error::NotFound | Error::InvalidInput));
        if new_dir.is_ok() {
            let new_dir = new_dir.unwrap();
            let new_dir = Arc::new(Mutex::new(new_dir));
            let inode = FatFsDirInode::new(
                &self.dir,
                new_dir,
                &self.attr.sb.upgrade().unwrap(),
                VfsNodePerm::default_dir(),
            );
            let inode = Arc::new(inode);
            inode_cache.insert(name.to_string(), inode.clone());
            return Ok(Some(inode));
        }
        if new_dir.err().unwrap() {
            return Err(VfsError::IoError);
        }
        let file = dir.open_file(name).map_err(|e| match e {
            Error::NotFound | Error::InvalidInput => VfsError::NoEntry,
            _ => VfsError::IoError,
        })?;
        let file = Arc::new(Mutex::new(file));
        let inode = FatFsFileInode::new(
            &self.dir,
            file,
            &self.attr.sb.upgrade().unwrap(),
            name.to_string(),
            VfsNodePerm::default_file(),
        );
        let inode = Arc::new(inode);
        inode_cache.insert(name.to_string(), inode.clone());
        Ok(Some(inode))
    }

    fn rmdir(&self, name: &str) -> VfsResult<()> {
        self.delete_file(name, VfsNodeType::Dir)
    }
    fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        Ok(())
    }

    fn get_attr(&self) -> VfsResult<FileStat> {
        let attr = self.attr.inner.lock();

        Ok(FileStat {
            st_dev: 0,
            st_ino: 1,
            st_mode: attr.perm.bits() as u32,
            st_nlink: 1,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size: 4096,
            st_blksize: 512,
            __pad2: 0,
            st_blocks: 0,
            st_atime: attr.atime,
            st_mtime: attr.mtime,
            st_ctime: attr.ctime,
            unused: 0,
        })
    }

    fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::Dir
    }
}
