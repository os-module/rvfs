use crate::UniFsSuperBlock;
use crate::*;
use alloc::string::String;
use alloc::sync::Weak;
use alloc::vec::Vec;
use vfscore::inode::InodeAttr;
use vfscore::utils::{FileStat, VfsDirEntry, VfsNodePerm, VfsNodeType};

pub struct UniFsInodeSame<T: Send + Sync, R: VfsRawMutex> {
    pub sb: Weak<UniFsSuperBlock<R>>,
    pub inode_number: u64,
    pub provider: T,
    pub inner: lock_api::Mutex<R, UniFsInodeAttr>,
}

pub struct UniFsInodeAttr {
    pub link_count: u32,
    pub atime: VfsTimeSpec,
    pub mtime: VfsTimeSpec,
    pub ctime: VfsTimeSpec,
    pub perm: VfsNodePerm,
}

pub fn basic_file_stat<T: Send + Sync, R: VfsRawMutex>(basic: &UniFsInodeSame<T, R>) -> FileStat {
    let inner = basic.inner.lock();
    FileStat {
        st_dev: 0,
        st_ino: basic.inode_number,
        st_mode: inner.perm.bits() as u32,
        st_nlink: inner.link_count,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        __pad: 0,
        st_size: 4096,
        st_blksize: 4096,
        __pad2: 0,
        st_blocks: 0,
        st_atime: inner.atime,
        st_mtime: inner.mtime,
        st_ctime: inner.ctime,
        unused: 0,
    }
}

pub struct UniFsDirInode<T: Send + Sync, R: VfsRawMutex> {
    pub basic: UniFsInodeSame<T, R>,
    pub children: lock_api::Mutex<R, Vec<(String, u64)>>,
}

impl<T: Send + Sync + 'static, R: VfsRawMutex + 'static> UniFsDirInode<T, R> {
    pub fn readdir(&self, start_index: usize) -> VfsResult<Option<VfsDirEntry>> {
        let sb = self.basic.sb.upgrade().unwrap();
        let children = self.children.lock();
        let res = children
            .iter()
            .nth(start_index)
            .map(|(name, inode_number)| {
                let inode = sb
                    .get_inode(*inode_number)
                    .unwrap_or_else(|| panic!("inode {} not found in superblock", inode_number,));
                VfsDirEntry {
                    ino: *inode_number,
                    ty: inode.inode_type(),
                    name: name.clone(),
                }
            });
        Ok(res)
    }

    pub fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        let res = self.basic.sb.upgrade().ok_or(VfsError::Invalid);
        res.map(|sb| sb as Arc<dyn VfsSuperBlock>)
    }

    pub fn lookup(&self, name: &str) -> VfsResult<Option<Arc<dyn VfsInode>>> {
        let sb = self.basic.sb.upgrade().unwrap();
        let res = self
            .children
            .lock()
            .iter()
            .find(|(item_name, _item)| item_name.as_str() == name)
            .map(|(_, inode_number)| sb.get_inode(*inode_number));
        if let Some(res) = res {
            Ok(res)
        } else {
            Ok(None)
        }
    }

    #[inline]
    pub fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        Ok(())
    }
    #[inline]
    pub fn get_attr(&self) -> VfsResult<FileStat> {
        Ok(basic_file_stat(&self.basic))
    }
    #[inline]
    pub fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::Dir
    }
}
