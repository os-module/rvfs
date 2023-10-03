use crate::VfsRawMutex;
use crate::{KernelProvider, RamFsSuperBlock};

use alloc::string::{String, ToString};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use vfscore::dentry::VfsDentry;
use vfscore::error::VfsError;
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{FileStat, VfsDirEntry, VfsNodePerm, VfsNodeType, VfsTimeSpec};
use vfscore::VfsResult;
pub struct RamFsFileInode<T:Send+Sync, R: VfsRawMutex> {
    sb: Weak<RamFsSuperBlock<T, R>>,
    provider: T,
    inode_number: u64,
    inner: lock_api::Mutex<R, RamFsFileInodeInner>,
}
struct RamFsFileInodeInner {
    data: Vec<u8>,
    atime: VfsTimeSpec,
    mtime: VfsTimeSpec,
    ctime: VfsTimeSpec,
    perm: VfsNodePerm,
}

pub struct RamFsDirInode<T:Send+Sync, R: VfsRawMutex> {
    sb: Weak<RamFsSuperBlock<T, R>>,
    provider: T,
    inode_number: u64,
    children: lock_api::Mutex<R, Vec<(String, u64)>>,
    perm:lock_api::Mutex<R, VfsNodePerm>,
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> RamFsFileInode<T, R> {
    pub fn new(sb:&Arc<RamFsSuperBlock<T,R>>,provider: T, inode_number: u64, perm:VfsNodePerm) -> Self {
        let time = provider.current_time();
        Self {
            sb:Arc::downgrade(sb),
            provider,
            inode_number,
            inner: lock_api::Mutex::new(RamFsFileInodeInner {
                data: Vec::new(),
                atime: time,
                mtime: time,
                ctime: time,
                perm,
            }),
        }
    }
    pub fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let inner = self.inner.lock();
        let size = inner.data.len() as u64;
        let offset = offset.min(size);
        let len = (size - offset).min(buf.len() as u64) as usize;
        let data = inner.data.as_slice();
        buf[..len].copy_from_slice(&data[offset as usize..offset as usize + len]);
        Ok(len)
    }
    pub fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
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
    pub fn size(&self) -> u64 {
        self.inner.lock().data.len() as u64
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> RamFsDirInode<T, R> {
    pub fn new(sb:&Arc<RamFsSuperBlock<T,R>>,provider: T, inode_number: u64,perm:VfsNodePerm) -> Self {
        Self {
            sb: Arc::downgrade(sb),
            provider,
            inode_number,
            children: lock_api::Mutex::new(Vec::new()),
            perm:lock_api::Mutex::new(perm),
        }
    }

    pub fn read_dir(&self,start:usize)->Option<VfsDirEntry>{
        let ramfs_sb = self.sb.upgrade().unwrap();
        let children = self.children.lock();
        children.iter().skip(start).next().map(|(name,inode_number)|{
            let inode = ramfs_sb.get_inode(*inode_number).unwrap();
            VfsDirEntry{
                ino: *inode_number,
                ty: inode.inode_type(),
                name: name.clone(),
            }
        })
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsInode for RamFsDirInode<T, R> {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        let res = self.sb.upgrade().unwrap();
        Ok(res)
    }

    fn create(
        &self,
        name: &str,
        ty: VfsNodeType,
        perm: VfsNodePerm,
        _rdev: Option<u32>,
    ) -> VfsResult<Arc<dyn VfsInode>> {
        let sb = self.get_super_block()?.downcast_arc::<RamFsSuperBlock<T, R>>().unwrap();
        let inode_number = sb
            .inode_index
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);

        let inode:Arc<dyn VfsInode>  = match ty {
            VfsNodeType::File =>{
                Arc::new(RamFsFileInode::<_, R>::new(&sb,self.provider.clone(), inode_number,perm))
            }
            VfsNodeType::Dir =>{
                Arc::new(RamFsDirInode::<_, R>::new(&sb,self.provider.clone(), inode_number,perm))
            }
            _ => {
                return Err(VfsError::Invalid);
            }
        };
        sb.insert_inode(inode_number, inode.clone());
        self.children.lock().push((name.to_string(), inode_number));
        Ok(inode)
    }
    fn symlink(&self, _name: &str, _syn_name: &str) -> VfsResult<Arc<dyn VfsDentry>> {
        todo!()
    }
    fn lookup(
        &self,
        name: &str,
    ) -> VfsResult<Option<Arc<dyn VfsInode>>> {
        let ramfs_sb = self.sb.upgrade().unwrap();
        let res = self
            .children
            .lock()
            .iter()
            .find(|(item_name, _item)| item_name.as_str() == name)
            .map(|(_, inode_number)| ramfs_sb.get_inode(*inode_number));
        if let Some(res) = res {
            Ok(res)
        } else {
            Ok(None)
        }
    }
    fn rmdir(&self, _target: Arc<dyn VfsDentry>) -> VfsResult<()> {
        todo!()
    }
    fn set_attr(&self, _target: Arc<dyn VfsDentry>, _attr: InodeAttr) -> VfsResult<()> {
        todo!()
    }
    fn get_attr(&self, _target: Arc<dyn VfsDentry>) -> VfsResult<FileStat> {
        Ok(FileStat {
            st_dev: 0,
            st_ino: self.inode_number,
            st_mode: 0,
            st_nlink: 0,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size: 4096,
            st_blksize: 4096,
            __pad2: 0,
            st_blocks: 0,
            st_atime: VfsTimeSpec::new(0, 0),
            st_mtime: VfsTimeSpec::new(0, 0),
            unused: 0,
            st_ctime: VfsTimeSpec::new(0, 0),
        })
    }
    fn list_xattr(&self, _target: Arc<dyn VfsDentry>) -> VfsResult<Vec<String>> {
        todo!()
    }

    fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::Dir
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsInode for RamFsFileInode<T, R> {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        let res = self.sb.upgrade().unwrap();
        Ok(res)
    }

    fn set_attr(&self, _target: Arc<dyn VfsDentry>, _attr: InodeAttr) -> VfsResult<()> {
        let atime = self.provider.current_time();
        self.inner.lock().atime = atime;
        Ok(())
    }

    fn get_attr(&self, _target: Arc<dyn VfsDentry>) -> VfsResult<FileStat> {
        let inner = self.inner.lock();
        Ok(FileStat {
            st_dev: 0,
            st_ino: self.inode_number,
            st_mode: 0,
            st_nlink: 0,
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
            unused: 0,
            st_ctime: inner.ctime,
        })
    }

    fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::File
    }
}
