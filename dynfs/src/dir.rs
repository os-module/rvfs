use crate::file::DynFsFileInode;
use crate::*;
use alloc::string::ToString;
use alloc::vec::Vec;
use unifs::inode::UniFsDirInode;
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::inode::InodeAttr;
use vfscore::utils::{FileStat, VfsDirEntry, VfsNodePerm, VfsNodeType};

pub struct DynFsDirInode<T: Send + Sync, R: VfsRawMutex>(UniFsDirInode<T, R>);

impl<T: DynFsKernelProvider + 'static, R: VfsRawMutex + 'static> DynFsDirInode<T, R> {
    pub fn new(
        inode_number: u64,
        provider: T,
        sb: &Arc<UniFsSuperBlock<R>>,
        perm: VfsNodePerm,
    ) -> Self {
        Self(UniFsDirInode {
            basic: UniFsInodeSame::new(sb, provider, inode_number, perm),
            children: lock_api::Mutex::new(Vec::new()),
        })
    }

    fn add_manually(
        &self,
        ty: VfsNodeType,
        name: &str,
        inode: Option<Arc<dyn VfsInode>>,
        perm: VfsNodePerm,
    ) -> VfsResult<()> {
        let sb = self.0.basic.sb.upgrade().unwrap();
        let inode_number = sb
            .inode_index
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        sb.inode_count
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);

        let res: Arc<dyn VfsInode> = match ty {
            VfsNodeType::File => Arc::new(DynFsFileInode::new(
                &sb,
                self.0.basic.provider.clone(),
                inode_number,
                inode.unwrap(),
                perm,
            )) as _,
            VfsNodeType::Dir => Arc::new(DynFsDirInode::new(
                inode_number,
                self.0.basic.provider.clone(),
                &sb,
                perm,
            )),
            _ => return Err(VfsError::NoSys),
        };
        sb.insert_inode(inode_number, res);
        self.0
            .children
            .lock()
            .push((name.to_string(), inode_number));
        Ok(())
    }
    pub fn add_file_manually(
        &self,
        name: &str,
        inode: Arc<dyn VfsInode>,
        perm: VfsNodePerm,
    ) -> VfsResult<()> {
        self.add_manually(VfsNodeType::File, name, Some(inode), perm)
    }

    pub fn add_dir_manually(&self, name: &str, perm: VfsNodePerm) -> VfsResult<()> {
        self.add_manually(VfsNodeType::Dir, name, None, perm)
    }

    pub fn remove_manually(&self, name: &str) -> VfsResult<()> {
        let mut children = self.0.children.lock();
        let index = children.iter().position(|(n, _)| n == name).unwrap();
        let (_, inode_number) = children.remove(index);
        let sb = self.0.basic.sb.upgrade().unwrap();
        sb.remove_inode(inode_number);
        Ok(())
    }
}

impl<T: Send + Sync + 'static, R: VfsRawMutex + 'static> VfsFile for DynFsDirInode<T, R> {
    fn readdir(&self, start_index: usize) -> VfsResult<Option<VfsDirEntry>> {
        self.0.readdir(start_index)
    }
}

impl<T: DynFsKernelProvider + 'static, R: VfsRawMutex + 'static> VfsInode for DynFsDirInode<T, R> {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        self.0.get_super_block()
    }
    fn create(
        &self,
        _name: &str,
        _ty: VfsNodeType,
        _perm: VfsNodePerm,
        _rdev: Option<u32>,
    ) -> VfsResult<Arc<dyn VfsInode>> {
        Err(VfsError::NoSys)
    }

    fn lookup(&self, name: &str) -> VfsResult<Option<Arc<dyn VfsInode>>> {
        self.0.lookup(name)
    }

    fn set_attr(&self, attr: InodeAttr) -> VfsResult<()> {
        self.0.set_attr(attr)
    }

    fn get_attr(&self) -> VfsResult<FileStat> {
        self.0.get_attr()
    }

    fn inode_type(&self) -> VfsNodeType {
        self.0.inode_type()
    }
}
