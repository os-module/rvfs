use vfscore::dentry::VfsDentry;
use vfscore::inode::{InodeAttr, VfsInode};
use crate::{KernelProvider, RamFsSuperBlock};
use vfscore::{VfsResult};
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use vfscore::error::VfsError;
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{FileStat, VfsInodeMode, VfsNodeType, VfsTimeSpec};
use crate::VfsRawMutex;
pub struct RamFsFileInode<T, R: VfsRawMutex> {
    provider: T,
    inode_number:u64,
    inner: lock_api::Mutex<R, RamFsFileInodeInner>,

}
struct RamFsFileInodeInner {
    data: Vec<u8>,
    atime: VfsTimeSpec,
    mtime: VfsTimeSpec,
    ctime: VfsTimeSpec,
}

pub struct RamFsDirInode<T, R: VfsRawMutex> {
    provider: T,
    inode_number:u64,
    children: lock_api::Mutex<R,BTreeMap<String, u64>>,
}



impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> RamFsFileInode<T, R> {
    pub fn new(provider: T,inode_number:u64) -> Self {
        let time = provider.current_time();
        Self {
            provider,
            inode_number,
            inner: lock_api::Mutex::new(RamFsFileInodeInner {
                data: Vec::new(),
                atime: time,
                mtime: time,
                ctime: time,
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
    pub fn new(provider: T,inode_number:u64) -> Self {
        Self {
            provider,
            inode_number,
            children: lock_api::Mutex::new(BTreeMap::new()),
        }
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsInode for RamFsDirInode<T, R> {
    fn create(&self,name: &str,_mode: VfsInodeMode,sb:Arc<dyn VfsSuperBlock>) -> VfsResult<Arc<dyn VfsInode>> {
        let sb = sb.downcast_arc::<RamFsSuperBlock<T, R>>().unwrap();
        let inode_number = sb.inode_index.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        let inode = Arc::new(RamFsFileInode::<_,R>::new(self.provider.clone(),inode_number));
        sb.insert_inode(inode_number, inode.clone());
        self.children.lock().insert(name.to_string(), inode_number);
        Ok(inode)
    }
    fn symlink(&self, _name: &str, _syn_name: &str) -> VfsResult<Arc<dyn VfsDentry>> {
        todo!()
    }
    fn lookup(&self, name: &str,sb:Arc<dyn VfsSuperBlock>) -> VfsResult<Option<Arc<dyn VfsInode>>> {
        let ramfs_sb = sb.downcast_arc::<RamFsSuperBlock<T, R>>().unwrap();
        let res = self.children.lock().iter().find(|(item_name,_item)|item_name.as_str()==name)
            .map(|(_,&inode_number)|{
               ramfs_sb.get_inode(inode_number)
            });
        if let Some(res) = res {
            Ok(res)
        } else {
            Ok(None)
        }
    }
    fn mkdir(&self, _name: &str, _mode: u32) -> VfsResult<Arc<dyn VfsDentry>> {
        todo!()
    }
    fn rmdir(&self, _target: Arc<dyn VfsDentry>) -> VfsResult<()> {
        todo!()
    }
    fn set_attr(&self, _target: Arc<dyn VfsDentry>, _attr: InodeAttr) -> VfsResult<()> {
        todo!()
    }
    fn get_attr(&self, _target: Arc<dyn VfsDentry>) -> VfsResult<FileStat> {
        Ok(FileStat{
            st_dev: 0,
            st_ino: self.inode_number,
            st_mode: 0,
            st_nlink: 0,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size:4096,
            st_blksize: 4096,
            __pad2: 0,
            st_blocks: 0,
            st_atime: VfsTimeSpec::new(0,0),
            st_mtime: VfsTimeSpec::new(0,0),
            unused: 0,
            st_ctime: VfsTimeSpec::new(0,0),
        })
    }
    fn list_xattr(&self, _target: Arc<dyn VfsDentry>) -> VfsResult<Vec<String>> {
        todo!()
    }

    fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::Dir
    }
}


impl <T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsInode for RamFsFileInode<T,R>{
    fn create(&self, _name: &str, _mode: VfsInodeMode, _sb: Arc<dyn VfsSuperBlock>) -> VfsResult<Arc<dyn VfsInode>> {
        Err(VfsError::NoSys)
    }

    fn lookup(&self, _name: &str, _sb: Arc<dyn VfsSuperBlock>) -> VfsResult<Option<Arc<dyn VfsInode>>> {
        Err(VfsError::NoSys)
    }

    fn mkdir(&self, _name: &str, _mode: u32) -> VfsResult<Arc<dyn VfsDentry>> {
        Err(VfsError::NoSys)
    }

    fn rmdir(&self, _target: Arc<dyn VfsDentry>) -> VfsResult<()> {
        Err(VfsError::NoSys)
    }

    fn set_attr(&self, _target: Arc<dyn VfsDentry>, _attr: InodeAttr) -> VfsResult<()> {
        let atime = self.provider.current_time();
        self.inner.lock().atime = atime;
        Ok(())
    }

    fn get_attr(&self, _target: Arc<dyn VfsDentry>) -> VfsResult<FileStat> {
        let inner = self.inner.lock();
        Ok(FileStat{
            st_dev: 0,
            st_ino: self.inode_number,
            st_mode: 0,
            st_nlink: 0,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size:4096,
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