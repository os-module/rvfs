use vfscore::dentry::VfsDentry;
use vfscore::error::VfsError;
use vfscore::file::{SeekFrom, VfsFile};
use vfscore::fstype::{MountFlags, VfsMountPoint};
use vfscore::inode::VfsInode;
use crate::{KernelProvider, RamFsDirInode, RamFsFileInode, RamFsSuperBlock};
use vfscore::superblock::VfsSuperBlock;
use vfscore::{VfsResult};
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::sync::Weak;
use vfscore::utils::VfsNodeType;
use crate::VfsRawMutex;

pub struct RamFsDentry<T: Send + Sync, R: VfsRawMutex> {
    sb: Weak<RamFsSuperBlock<T, R>>,
    inner: lock_api::Mutex<R, RamFsDentryInner<T, R>>,
}

struct RamFsDentryInner<T: Send + Sync, R: VfsRawMutex> {
    parent: Weak<RamFsDentry<T, R>>,
    inode: Arc<dyn VfsInode>,
    name: String,
    mnt: Option<VfsMountPoint>,
    children: Option<BTreeMap<String, Arc<RamFsDentry<T, R>>>>,
}


impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> RamFsDentry<T, R> {
    /// Create the root dentry
    ///
    /// Only call once
    pub fn root(provider: T, sb: &Arc<RamFsSuperBlock<T, R>>) -> Self {
        sb.inode_count.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        let inode_number = sb.inode_index.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        let dentry = Self {
            sb: Arc::downgrade(sb),
            inner: lock_api::Mutex::new(RamFsDentryInner {
                parent: Weak::new(),
                inode: Arc::new(RamFsDirInode::<_, R>::new(provider,inode_number)),
                name: "/".to_string(),
                mnt: None,
                children: Some(BTreeMap::new()),
            }),
        };
        dentry
    }
    pub fn file_inode(&self) -> Arc<RamFsFileInode<T, R>> {
        self.inner
            .lock()
            .inode
            .clone()
            .downcast_arc::<RamFsFileInode<T, R>>()
            .unwrap()
    }

    pub fn dir_inode(&self) -> Arc<RamFsDirInode<T, R>> {
        self.inner
            .lock()
            .inode
            .clone()
            .downcast_arc::<RamFsDirInode<T, R>>()
            .unwrap()
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsDentry for RamFsDentry<T, R> {
    fn name(&self) -> String {
        self.inner.lock().name.clone()
    }

    fn to_mount_point(
        self: Arc<Self>,
        sub_fs_root: Arc<dyn VfsDentry>,
        mount_flag: MountFlags,
    ) -> VfsResult<()> {
        let point = self as Arc<dyn VfsDentry>;
        let mnt = VfsMountPoint {
            root: sub_fs_root.clone(),
            mount_point: Arc::downgrade(&point),
            mnt_flags: mount_flag,
        };
        let point = point.downcast_arc::<RamFsDentry<T, R>>().unwrap();
        let mut inner = point.inner.lock();
        inner.mnt = Some(mnt);
        Ok(())
    }

    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        let res = self.sb.upgrade().unwrap();
        Ok(res)
    }

    fn get_inode(&self) -> VfsResult<Arc<dyn VfsInode>> {
        Ok(self.inner.lock().inode.clone())
    }

    fn get_vfs_mount(&self) -> Option<VfsMountPoint> {
        self.inner
            .lock()
            .mnt
            .clone()
    }

    fn find(&self, path: &str) -> Option<Arc<dyn VfsDentry>> {
        let inner = self.inner.lock();
        let inode_type = inner.inode.inode_type();
        match inode_type {
            VfsNodeType::Dir => {
                inner.children.as_ref().unwrap().get(path)
                    .map(|item|item.clone() as Arc<dyn VfsDentry>)
            }
            _ => None
        }
    }

    fn insert(self:Arc<Self>, name:&str, child: Arc<dyn VfsInode>) -> VfsResult<Arc<dyn VfsDentry>> {
        let inode_type = child.inode_type();
        let child = Arc::new(RamFsDentry {
            sb: self.sb.clone(),
            inner: lock_api::Mutex::new(RamFsDentryInner {
                parent: Arc::downgrade(&self),
                inode: child,
                name: name.to_string(),
                mnt: None,
                children: match inode_type {
                    VfsNodeType::Dir => Some(BTreeMap::new()),
                    _ => None
                },
            }),
        });
        self.inner.lock().children.as_mut().unwrap().insert(name.to_string(), child.clone())
            .map_or(Ok(child),|_|Err(VfsError::FileExist))
    }
}

pub struct RamFsFile<T: Send + Sync, R: VfsRawMutex> {
    dentry: Arc<RamFsDentry<T, R>>,
    inner: lock_api::Mutex<R, RamFsFileInner>,
}

struct RamFsFileInner {
    offset: u64,
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> RamFsFile<T, R> {
    pub fn new(dentry: Arc<RamFsDentry<T, R>>) -> Self {
        Self {
            dentry,
            inner: lock_api::Mutex::new(RamFsFileInner { offset: 0 }),
        }
    }
}

impl<T: KernelProvider + 'static, R: VfsRawMutex + 'static> VfsFile for RamFsFile<T, R> {
    fn seek(&self, pos: SeekFrom) -> VfsResult<u64> {
        let seek = || {
            let mut inner = self.inner.lock();
            let size = self.dentry.file_inode().size();
            let new_offset = match pos {
                SeekFrom::Start(pos) => Some(pos),
                SeekFrom::Current(off) => inner.offset.checked_add_signed(off),
                SeekFrom::End(off) => size.checked_add_signed(off),
            }
            .ok_or_else(|| VfsError::Invalid)?;
            inner.offset = new_offset;
            Ok(inner.offset)
        };
        match self.dentry.get_inode()?.inode_type() {
            VfsNodeType::File => seek(),
            _ => return Err(VfsError::Invalid),
        }
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        match self.dentry.get_inode()?.inode_type() {
            VfsNodeType::File => self.dentry.file_inode().read_at(offset, buf),
            _ => return Err(VfsError::Invalid),
        }
    }
    fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        match self.dentry.get_inode()?.inode_type() {
            VfsNodeType::File => self.dentry.file_inode().write_at(offset, buf),
            _ => return Err(VfsError::Invalid),
        }
    }
}
