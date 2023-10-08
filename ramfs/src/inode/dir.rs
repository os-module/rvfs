use super::*;
use crate::inode::file::RamFsFileInode;
use crate::inode::symlink::RamFsSymLinkInode;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use unifs::inode::{basic_file_stat, UniFsDirInode};
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{VfsDirEntry, VfsNodePerm, VfsNodeType};
use vfscore::VfsResult;
pub struct RamFsDirInode<T: Send + Sync, R: VfsRawMutex> {
    inode: UniFsDirInode<T, R>,
    ext_attr: lock_api::Mutex<R, BTreeMap<String, String>>,
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> RamFsDirInode<T, R> {
    pub fn new(
        sb: &Arc<UniFsSuperBlock<R>>,
        provider: T,
        inode_number: u64,
        perm: VfsNodePerm,
    ) -> Self {
        Self {
            inode: UniFsDirInode {
                basic: UniFsInodeSame::new(sb, provider, inode_number, perm),
                children: lock_api::Mutex::new(Vec::new()),
            },
            ext_attr: lock_api::Mutex::new(BTreeMap::new()),
        }
    }
    pub fn update_metadata<F, Res>(&self, f: F) -> Res
    where
        F: FnOnce(&UniFsInodeSame<T, R>) -> Res,
    {
        f(&self.inode.basic)
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsFile for RamFsDirInode<T, R> {
    fn readdir(&self, start_index: usize) -> VfsResult<Option<VfsDirEntry>> {
        self.inode.readdir(start_index)
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsInode for RamFsDirInode<T, R> {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        self.inode.get_super_block()
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
            .downcast_arc::<UniFsSuperBlock<R>>()
            .unwrap();
        let inode_number = sb
            .inode_index
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);

        let inode: Arc<dyn VfsInode> = match ty {
            VfsNodeType::File => Arc::new(RamFsFileInode::<_, R>::new(
                &sb,
                self.inode.basic.provider.clone(),
                inode_number,
                perm,
            )),
            VfsNodeType::Dir => Arc::new(RamFsDirInode::<_, R>::new(
                &sb,
                self.inode.basic.provider.clone(),
                inode_number,
                perm,
            )),
            _ => {
                return Err(VfsError::Invalid);
            }
        };
        sb.insert_inode(inode_number, inode.clone());
        self.inode
            .children
            .lock()
            .push((name.to_string(), inode_number));
        Ok(inode)
    }
    fn link(&self, name: &str, src: Arc<dyn VfsInode>) -> VfsResult<Arc<dyn VfsInode>> {
        let sb = self
            .get_super_block()?
            .downcast_arc::<UniFsSuperBlock<R>>()
            .unwrap();
        sb.inode_count
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);

        let inode = src.downcast_arc::<RamFsFileInode<T, R>>().unwrap();

        let inode_number = inode.update_metadata(|meta| {
            meta.inner.lock().link_count += 1;
            meta.inode_number
        });
        self.inode
            .children
            .lock()
            .push((name.to_string(), inode_number));

        Ok(inode)
    }

    fn unlink(&self, _name: &str) -> VfsResult<()> {
        todo!()
    }

    fn symlink(&self, name: &str, sy_name: &str) -> VfsResult<Arc<dyn VfsInode>> {
        let sb = self
            .get_super_block()?
            .downcast_arc::<UniFsSuperBlock<R>>()
            .unwrap();
        let inode_number = sb
            .inode_index
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        let inode = Arc::new(RamFsSymLinkInode::<_, R>::new(
            &sb,
            self.inode.basic.provider.clone(),
            inode_number,
            sy_name.to_string(),
        ));
        self.inode
            .children
            .lock()
            .push((name.to_string(), inode_number));
        sb.insert_inode(inode_number, inode.clone());
        Ok(inode)
    }
    fn lookup(&self, name: &str) -> VfsResult<Option<Arc<dyn VfsInode>>> {
        self.inode.lookup(name)
    }

    fn rmdir(&self, _name: &str) -> VfsResult<()> {
        todo!()
    }

    fn set_attr(&self, attr: InodeAttr) -> VfsResult<()> {
        set_attr(&self.inode.basic, attr);
        Ok(())
    }
    fn get_attr(&self) -> VfsResult<FileStat> {
        let mut stat = basic_file_stat(&self.inode.basic);
        stat.st_size = 4096;
        Ok(stat)
    }
    fn list_xattr(&self) -> VfsResult<Vec<String>> {
        let res = self.ext_attr.lock().keys().cloned().collect();
        Ok(res)
    }

    fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::Dir
    }
}
