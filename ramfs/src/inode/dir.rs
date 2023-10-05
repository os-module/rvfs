use super::*;
use crate::inode::file::RamFsFileInode;
use crate::inode::symlink::RamFsSymLinkInode;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{VfsDirEntry, VfsNodePerm, VfsNodeType};
use vfscore::VfsResult;
pub struct RamFsDirInode<T: Send + Sync, R: VfsRawMutex> {
    basic: RamfsInodeSame<T, R>,
    children: lock_api::Mutex<R, Vec<(String, u64)>>,
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> RamFsDirInode<T, R> {
    pub fn new(
        sb: &Arc<RamFsSuperBlock<T, R>>,
        provider: T,
        inode_number: u64,
        perm: VfsNodePerm,
    ) -> Self {
        Self {
            basic: RamfsInodeSame::new(sb, provider, inode_number, perm),
            children: lock_api::Mutex::new(Vec::new()),
        }
    }
    pub fn update_metadata<F, Res>(&self, f: F) -> Res
    where
        F: FnOnce(&RamfsInodeSame<T, R>) -> Res,
    {
        f(&self.basic)
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsFile for RamFsDirInode<T, R> {
    fn readdir(&self, start_index: usize) -> VfsResult<Option<VfsDirEntry>> {
        let ramfs_sb = self.basic.sb.upgrade().unwrap();
        let children = self.children.lock();
        let res = children
            .iter()
            .skip(start_index)
            .next()
            .map(|(name, inode_number)| {
                let inode = ramfs_sb.get_inode(*inode_number).unwrap();
                VfsDirEntry {
                    ino: *inode_number,
                    ty: inode.inode_type(),
                    name: name.clone(),
                }
            });
        Ok(res)
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsInode for RamFsDirInode<T, R> {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        let res = self.basic.sb.upgrade().unwrap();
        Ok(res)
    }
    fn create(
        &self,
        name: &str,
        ty: VfsNodeType,
        perm: VfsNodePerm,
        _rdev: Option<u32>,
    ) -> VfsResult<Arc<dyn VfsInode>> {
        let sb = self
            .get_super_block()?
            .downcast_arc::<RamFsSuperBlock<T, R>>()
            .unwrap();
        let inode_number = sb
            .inode_index
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);

        let inode: Arc<dyn VfsInode> = match ty {
            VfsNodeType::File => Arc::new(RamFsFileInode::<_, R>::new(
                &sb,
                self.basic.provider.clone(),
                inode_number,
                perm,
            )),
            VfsNodeType::Dir => Arc::new(RamFsDirInode::<_, R>::new(
                &sb,
                self.basic.provider.clone(),
                inode_number,
                perm,
            )),
            _ => {
                return Err(VfsError::Invalid);
            }
        };
        sb.insert_inode(inode_number, inode.clone());
        self.children.lock().push((name.to_string(), inode_number));
        Ok(inode)
    }
    fn link(&self, name: &str, src: Arc<dyn VfsInode>) -> VfsResult<Arc<dyn VfsInode>> {
        let sb = self
            .get_super_block()?
            .downcast_arc::<RamFsSuperBlock<T, R>>()
            .unwrap();
        let inode_number = sb
            .inode_index
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        self.children.lock().push((name.to_string(), inode_number));
        let inode = src.downcast_arc::<RamFsFileInode<T, R>>().unwrap();
        inode.update_metadata(|meta| {
            meta.inner.lock().link_count += 1;
        });
        Ok(inode)
    }

    fn unlink(&self, _name: &str) -> VfsResult<()> {
        todo!()
    }

    fn symlink(&self, name: &str, sy_name: &str) -> VfsResult<Arc<dyn VfsInode>> {
        let sb = self
            .get_super_block()?
            .downcast_arc::<RamFsSuperBlock<T, R>>()
            .unwrap();
        let inode_number = sb
            .inode_index
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        let inode = Arc::new(RamFsSymLinkInode::<_, R>::new(
            &sb,
            self.basic.provider.clone(),
            inode_number,
            sy_name.to_string(),
        ));
        sb.insert_inode(inode_number, inode.clone());
        self.children.lock().push((name.to_string(), inode_number));
        Ok(inode)
    }
    fn lookup(&self, name: &str) -> VfsResult<Option<Arc<dyn VfsInode>>> {
        let ramfs_sb = self.basic.sb.upgrade().unwrap();
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

    fn rmdir(&self, _name: &str) -> VfsResult<()> {
        todo!()
    }

    fn set_attr(&self, attr: InodeAttr) -> VfsResult<()> {
        set_attr(&self.basic, attr);
        Ok(())
    }
    fn get_attr(&self) -> VfsResult<FileStat> {
        let mut stat = basic_file_stat(&self.basic);
        stat.st_size = 4096;
        Ok(stat)
    }
    fn list_xattr(&self) -> VfsResult<Vec<String>> {
        let res = self
            .basic
            .inner
            .lock()
            .ext_attr
            .keys()
            .map(|k| k.clone())
            .collect();
        Ok(res)
    }

    fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::Dir
    }
}
