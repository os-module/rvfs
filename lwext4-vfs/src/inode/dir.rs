use alloc::{
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};

use lock_api::Mutex;
use log::{debug, info, warn};
use lwext4_rs::{FileTimes, MetaDataExt, Permissions, ReadDir, Time};
use vfscore::{
    error::VfsError,
    file::VfsFile,
    impl_dir_inode_default,
    inode::{InodeAttr, VfsInode},
    superblock::VfsSuperBlock,
    utils::{
        VfsDirEntry, VfsFileStat, VfsNodePerm, VfsNodeType, VfsRenameFlag, VfsTime, VfsTimeSpec,
    },
    VfsResult,
};

use crate::{
    inode::{file::ExtFileInode, link::ExtLinkInode, special::ExtSpecialInode, ExtFsInodeAttr},
    types::{into_file_type, into_vfs, into_vfs_node_type, Parent, ToDir},
    ExtFsSuperBlock, VfsRawMutex,
};

pub struct ExtDirInode<R: VfsRawMutex> {
    dir: Arc<Mutex<R, ReadDir>>,
    sb: Weak<ExtFsSuperBlock<R>>,
    times: Mutex<R, ExtFsInodeAttr>,
}
unsafe impl<R: VfsRawMutex> Send for ExtDirInode<R> {}
unsafe impl<R: VfsRawMutex> Sync for ExtDirInode<R> {}
impl<R: VfsRawMutex> ExtDirInode<R> {
    pub(crate) fn new(dir: Arc<Mutex<R, ReadDir>>, sb: &Arc<ExtFsSuperBlock<R>>) -> Self {
        Self {
            dir,
            sb: Arc::downgrade(sb),
            times: Mutex::new(ExtFsInodeAttr::default()),
        }
    }
    fn path(&self) -> String {
        self.dir.lock().path()
    }
}

impl<R: VfsRawMutex + 'static> VfsFile for ExtDirInode<R> {
    fn readdir(&self, start_index: usize) -> VfsResult<Option<VfsDirEntry>> {
        let mut dir = self.dir.lock();
        // todo!(This should be optimized)
        dir.rewind();
        dir.advance_by(start_index).map_err(|_| VfsError::NoEntry)?;
        let entry = dir.next();
        let entry = match entry {
            None => return Ok(None),
            Some(entry) => entry,
        };
        let ty = entry.file_type().map_err(into_vfs)?;
        Ok(Some(VfsDirEntry {
            ino: entry.inode() as u64,
            ty: into_vfs_node_type(ty),
            name: entry.name().to_string(),
        }))
    }
    fn ioctl(&self, _cmd: u32, _arg: usize) -> VfsResult<usize> {
        Err(VfsError::NoTTY)
    }
}

impl<R: VfsRawMutex + 'static> VfsInode for ExtDirInode<R> {
    impl_dir_inode_default!();
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        Ok(self.sb.upgrade().unwrap())
    }
    fn node_perm(&self) -> VfsNodePerm {
        let file = self.dir.lock().as_file();
        let perm = file.metadata().map_or(VfsNodePerm::default_dir(), |meta| {
            VfsNodePerm::from_bits_truncate(meta.permissions().mode() as u16)
        });
        perm
    }
    fn create(
        &self,
        name: &str,
        ty: VfsNodeType,
        perm: VfsNodePerm,
        rdev: Option<u64>,
    ) -> VfsResult<Arc<dyn VfsInode>> {
        let sb = self
            .get_super_block()?
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)?;
        let path = self.path() + name;
        info!("[create] file path: {}, ty: {:?}", path, ty);
        match ty {
            VfsNodeType::File => {
                let file = sb
                    .fs
                    .file_builder()
                    .mode(perm.bits() as u32)
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path)
                    .map_err(into_vfs)?;
                let file = ExtFileInode::new(file, &sb);
                Ok(Arc::new(file) as Arc<dyn VfsInode>)
            }
            VfsNodeType::Dir => {
                sb.fs.create_dir(&path).map_err(into_vfs)?;
                let dir = sb.fs.readdir(path).map_err(into_vfs)?;
                let dir = ExtDirInode::new(Arc::new(Mutex::new(dir)), &sb);
                Ok(Arc::new(dir) as Arc<dyn VfsInode>)
            }
            VfsNodeType::SymLink => Err(VfsError::Invalid),
            VfsNodeType::BlockDevice | VfsNodeType::CharDevice => {
                let rdev = rdev.ok_or(VfsError::Invalid)?;
                sb.fs
                    .mknod(&path, into_file_type(ty)?, rdev as u32)
                    .map_err(into_vfs)?;
                sb.fs
                    .set_permissions(&path, Permissions::from_mode(perm.bits() as u32))
                    .map_err(into_vfs)?;
                let file = ExtSpecialInode::new(path, &sb, ty, rdev, sb.provider.clone());
                Ok(Arc::new(file) as Arc<dyn VfsInode>)
            }
            ty => {
                warn!("[create] unsupported file type: {:?}", ty);
                Err(VfsError::NoSys)
            }
        }
    }
    fn link(&self, name: &str, src: Arc<dyn VfsInode>) -> VfsResult<Arc<dyn VfsInode>> {
        let original = match src.inode_type() {
            VfsNodeType::File => {
                let file = src
                    .downcast_arc::<ExtFileInode<R>>()
                    .map_err(|_x| VfsError::Invalid)?;
                file.path()
            }
            VfsNodeType::Dir => {
                let dir = src
                    .downcast_arc::<ExtDirInode<R>>()
                    .map_err(|_x| VfsError::Invalid)?;
                dir.path()
            }
            VfsNodeType::SymLink => {
                let link = src
                    .downcast_arc::<ExtLinkInode<R>>()
                    .map_err(|_x| VfsError::Invalid)?;
                link.path()
            }
            VfsNodeType::CharDevice | VfsNodeType::BlockDevice => {
                let special = src
                    .downcast_arc::<ExtSpecialInode<R>>()
                    .map_err(|_x| VfsError::Invalid)?;
                special.path()
            }
            VfsNodeType::Socket | VfsNodeType::Fifo => {
                return Err(VfsError::NoSys);
            }
            _ => {
                return Err(VfsError::NoSys);
            }
        };
        let sb = self
            .get_super_block()?
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)?;
        let link = self.path() + name;
        info!("[link] name: {}, src: {:?}", link, original);
        sb.fs.hard_link(original, link).map_err(into_vfs)?;
        self.lookup(name)
    }
    fn unlink(&self, name: &str) -> VfsResult<()> {
        let sb = self
            .get_super_block()?
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)?;
        let path = self.path() + name;
        info!("[unlink] path: {}", path);
        sb.fs.remove_file(path).map_err(into_vfs)
    }
    fn symlink(&self, name: &str, sy_name: &str) -> VfsResult<Arc<dyn VfsInode>> {
        let sb = self
            .get_super_block()?
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)?;
        let path = self.path() + name;
        sb.fs.soft_link(&sy_name, &path).map_err(into_vfs)?;
        info!("[symlink] path: {} -> {}", path, sy_name);
        Ok(self.lookup(name)?)
    }
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn VfsInode>> {
        debug!("[extfs] lookup: {}", name);
        let mut dir = self.dir.lock();
        dir.rewind();
        let entry = dir
            .find(|entry| entry.name() == name)
            .ok_or(VfsError::NoEntry)?;
        debug!("entry: {:?}", entry);
        let sb = self
            .get_super_block()?
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)?;
        let ty = into_vfs_node_type(entry.file_type().map_err(into_vfs)?);
        match ty {
            VfsNodeType::Unknown => {
                warn!("[extfs] lookup: unknown file type {:?}", entry.file_type());
                Err(VfsError::Invalid)
            }
            VfsNodeType::Fifo | VfsNodeType::Socket => {
                unimplemented!()
            }
            VfsNodeType::CharDevice | VfsNodeType::BlockDevice => {
                let path = entry.path();
                let meta = sb.fs.metadata(&path).map_err(into_vfs)?;
                let file =
                    ExtSpecialInode::new(path, &sb, ty, meta.rdev() as _, sb.provider.clone());
                Ok(Arc::new(file) as Arc<dyn VfsInode>)
            }
            VfsNodeType::Dir => {
                let path = entry.path();
                if name == "." {
                    let dir = ExtDirInode::new(self.dir.clone(), &sb);
                    return Ok(Arc::new(dir) as Arc<dyn VfsInode>);
                } else if name == ".." {
                    let p_path = path.parent().unwrap();
                    return if p_path == "" {
                        let dir = ExtDirInode::new(self.dir.clone(), &sb);
                        Ok(Arc::new(dir) as Arc<dyn VfsInode>)
                    } else {
                        let dir = sb.fs.readdir(p_path).map_err(into_vfs)?;
                        let dir = ExtDirInode::new(Arc::new(Mutex::new(dir)), &sb);
                        Ok(Arc::new(dir) as Arc<dyn VfsInode>)
                    };
                } else {
                    let dir = sb.fs.readdir(path.to_dir()).map_err(into_vfs)?;
                    let dir = ExtDirInode::new(Arc::new(Mutex::new(dir)), &sb);
                    Ok(Arc::new(dir) as Arc<dyn VfsInode>)
                }
            }

            VfsNodeType::File => {
                let path = entry.path();
                let file = sb
                    .fs
                    .file_builder()
                    .read(true)
                    .write(true)
                    .open(path)
                    .map_err(into_vfs)?;
                let file = ExtFileInode::new(file, &sb);
                Ok(Arc::new(file) as Arc<dyn VfsInode>)
            }
            VfsNodeType::SymLink => {
                let path = entry.path();
                let file = ExtLinkInode::new(path, &sb);
                Ok(Arc::new(file) as Arc<dyn VfsInode>)
            }
        }
    }
    fn rmdir(&self, name: &str) -> VfsResult<()> {
        let sb = self
            .get_super_block()?
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)?;
        let path = self.path() + name;
        info!("[rm dir] path: {}", path);
        sb.fs.remove_dir(path).map_err(into_vfs)
    }
    fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        Ok(())
    }
    fn get_attr(&self) -> VfsResult<VfsFileStat> {
        let dir = self.dir.lock();
        let file = dir.as_file();
        let meta = file.metadata().map_err(into_vfs)?;
        let sb = self
            .get_super_block()?
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)?;
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
            st_size: 4096,
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
        VfsNodeType::Dir
    }
    fn rename_to(
        &self,
        old_name: &str,
        new_parent: Arc<dyn VfsInode>,
        new_name: &str,
        _flag: VfsRenameFlag,
    ) -> VfsResult<()> {
        let new_parent = new_parent
            .downcast_arc::<ExtDirInode<R>>()
            .map_err(|_x| VfsError::Invalid)?;
        let old_path = self.path() + old_name;
        let new_path = new_parent.path() + new_name;
        info!("[rename] old path: {}, new path: {}", old_path, new_path);
        let sb = self
            .get_super_block()?
            .downcast_arc::<ExtFsSuperBlock<R>>()
            .map_err(|_x| VfsError::Invalid)?;
        sb.fs.rename(old_path, new_path).map_err(into_vfs)
    }
    fn update_time(&self, time: VfsTime, now: VfsTimeSpec) -> VfsResult<()> {
        let dir = self.dir.lock();
        let mut file = dir.as_file();
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
        info!("[update_time] path: {:?}, times: {:?}", file.path(), times);
        attr_times.ctime = now;
        file.set_times(times).map_err(into_vfs)
    }
}
