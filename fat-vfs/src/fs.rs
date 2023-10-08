use super::*;
use crate::device::FatDevice;
use alloc::collections::BTreeMap;
use alloc::sync::Weak;
use fatfs::FileSystem;

use vfscore::dentry::VfsDentry;
use vfscore::error::VfsError;
use vfscore::fstype::{FileSystemFlags, MountFlags, VfsFsType};
use vfscore::inode::VfsInode;
use vfscore::superblock::{SuperType, VfsSuperBlock};
use vfscore::utils::VfsFsStat;
use vfscore::VfsResult;

pub struct FatFs<T: Send + Sync, R: VfsRawMutex> {
    provider: T,
    fs_container: Mutex<R, BTreeMap<usize, Arc<FatFsSuperBlock<R>>>>,
}

impl<T: Send + Sync, R: VfsRawMutex> FatFs<T, R> {
    pub fn new(provider: T) -> Self {
        Self {
            provider,
            fs_container: Mutex::new(BTreeMap::new()),
        }
    }
}

impl<T: FatFsProvider + 'static, R: VfsRawMutex + 'static> VfsFsType for FatFs<T, R> {
    fn mount(
        self: Arc<Self>,
        _flags: MountFlags,
        dev: Option<Arc<dyn VfsInode>>,
        _data: &[u8],
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        if dev.is_none() {
            return Err(VfsError::NoDev);
        }
        let fat_dev = FatDevice::new(dev.unwrap());
        let sb = FatFsSuperBlock::<R>::new(&(self as Arc<dyn VfsFsType>), fat_dev);
        sb.root_dentry()
    }

    fn kill_sb(&self, _sb: Arc<dyn VfsSuperBlock>) -> VfsResult<()> {
        todo!()
    }

    fn fs_flag(&self) -> FileSystemFlags {
        FileSystemFlags::REQUIRES_DEV
    }

    fn fs_name(&self) -> &'static str {
        "fatfs"
    }
}

pub struct FatFsSuperBlock<R: VfsRawMutex> {
    fs_type: Weak<dyn VfsFsType>,
    root: Mutex<R, Option<Arc<dyn VfsDentry>>>,
    fs: FileSystem<FatDevice, DefaultTimeProvider, LossyOemCpConverter>,
}

impl<R: VfsRawMutex> FatFsSuperBlock<R> {
    pub fn new(fs_type: &Arc<dyn VfsFsType>, device: FatDevice) -> Arc<Self> {
        let fs = FileSystem::new(device, fatfs::FsOptions::new()).unwrap();
        let sb = Arc::new(Self {
            fs_type: Arc::downgrade(fs_type),
            root: Mutex::new(None),
            fs,
        });
        sb
    }
}

impl<R: VfsRawMutex + 'static> VfsSuperBlock for FatFsSuperBlock<R> {
    fn sync_fs(&self, _wait: bool) -> VfsResult<()> {
        todo!()
    }

    fn stat_fs(&self) -> VfsResult<VfsFsStat> {
        todo!()
    }

    fn super_type(&self) -> SuperType {
        SuperType::BlockDev
    }

    fn fs_type(&self) -> Arc<dyn VfsFsType> {
        self.fs_type.upgrade().unwrap()
    }
    fn root_dentry(&self) -> VfsResult<Arc<dyn VfsDentry>> {
        todo!()
    }
}
