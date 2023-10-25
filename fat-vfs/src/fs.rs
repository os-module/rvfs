use super::*;
use crate::device::FatDevice;
use crate::inode::FatFsDirInode;
use alloc::collections::BTreeMap;
use alloc::sync::Weak;
use fatfs::FileSystem;
use log::info;
use unifs::dentry::UniFsDentry;
use vfscore::dentry::VfsDentry;
use vfscore::error::VfsError;
use vfscore::fstype::{FileSystemFlags, VfsFsType};
use vfscore::inode::VfsInode;
use vfscore::superblock::{SuperType, VfsSuperBlock};
use vfscore::utils::{VfsFsStat, VfsNodeType};
use vfscore::VfsResult;

pub struct FatFs<T: Send + Sync, R: VfsRawMutex> {
    #[allow(unused)]
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
        _flags: u32,
        dev: Option<Arc<dyn VfsInode>>,
        _data: &[u8],
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        if dev.is_none() {
            return Err(VfsError::NoDev);
        }
        let dev = dev.unwrap();
        if dev.inode_type() != VfsNodeType::BlockDevice {
            return Err(VfsError::Invalid);
        }
        let dev_ino = dev.get_attr()?.st_ino;

        if let Some(sb) = self.fs_container.lock().get(&(dev_ino as usize)) {
            return sb.root_dentry();
        }
        let fat_dev = FatDevice::new(dev);
        let sb = FatFsSuperBlock::<R>::new(&(self.clone() as Arc<dyn VfsFsType>), fat_dev);
        // we use dev_ino as the key to store the superblock
        self.fs_container
            .lock()
            .insert(dev_ino as usize, sb.clone());
        sb.root_dentry()
    }

    fn kill_sb(&self, sb: Arc<dyn VfsSuperBlock>) -> VfsResult<()> {
        if let Ok(sb) = sb.downcast_arc::<FatFsSuperBlock<R>>() {
            let dev_ino = sb.fat_dev.device_file.get_attr()?.st_ino;
            let sb = self.fs_container.lock().remove(&(dev_ino as usize));
            if let Some(sb) = sb {
                sb.fs.unmount().map_err(|_| VfsError::IoError)?;
                sb.fat_dev.device_file.fsync()?;
                info!("fatfs: kill_sb: remove sb for dev {}", dev_ino);
                Ok(())
            } else {
                Err(VfsError::Invalid)
            }
        } else {
            Err(VfsError::Invalid)
        }
    }

    fn fs_flag(&self) -> FileSystemFlags {
        FileSystemFlags::REQUIRES_DEV
    }

    fn fs_name(&self) -> &'static str {
        "fatfs"
    }
}

pub struct FatFsSuperBlock<R: VfsRawMutex> {
    fat_dev: FatDevice,
    fs_type: Weak<dyn VfsFsType>,
    root: Mutex<R, Option<Arc<dyn VfsDentry>>>,
    fs: FileSystem<FatDevice, DefaultTimeProvider, LossyOemCpConverter>,
}

impl<R: VfsRawMutex + 'static> FatFsSuperBlock<R> {
    pub fn new(fs_type: &Arc<dyn VfsFsType>, device: FatDevice) -> Arc<Self> {
        let fs = FileSystem::new(device.clone(), fatfs::FsOptions::new()).unwrap();
        let root_disk_dir = Arc::new(Mutex::new(fs.root_dir()));
        let sb = Arc::new(Self {
            fat_dev: device,
            fs_type: Arc::downgrade(fs_type),
            root: Mutex::new(None),
            fs,
        });
        let root_inode = FatFsDirInode::new(
            &root_disk_dir.clone(),
            root_disk_dir,
            &sb,
            "rwxrwxrwx".into(),
        );
        let root_inode = Arc::new(root_inode);
        let root_dt = Arc::new(UniFsDentry::<R>::root(root_inode));
        sb.root.lock().replace(root_dt);
        sb
    }
}

impl<R: VfsRawMutex + 'static> VfsSuperBlock for FatFsSuperBlock<R> {
    fn sync_fs(&self, _wait: bool) -> VfsResult<()> {
        todo!()
    }

    fn stat_fs(&self) -> VfsResult<VfsFsStat> {
        let stat_fs = self.fs.stats().map_err(|_| VfsError::IoError)?;
        let ft = self.fs.fat_type();
        let f_type = match ft {
            fatfs::FatType::Fat12 => 0x01,
            fatfs::FatType::Fat16 => 0x04,
            fatfs::FatType::Fat32 => 0x0c,
        };
        Ok(VfsFsStat {
            f_type,
            f_bsize: stat_fs.cluster_size() as i64,
            f_blocks: stat_fs.total_clusters() as u64,
            f_bfree: stat_fs.free_clusters() as u64,
            f_bavail: stat_fs.free_clusters() as u64,
            f_files: 0,
            f_ffree: 0,
            f_fsid: [0, 0],
            f_namelen: 0,
            f_frsize: 0,
            f_flags: 0,
            f_spare: [0; 4],
        })
    }

    fn super_type(&self) -> SuperType {
        SuperType::BlockDev
    }

    fn fs_type(&self) -> Arc<dyn VfsFsType> {
        self.fs_type.upgrade().unwrap()
    }
    fn root_dentry(&self) -> VfsResult<Arc<dyn VfsDentry>> {
        let root = self.root.lock().clone().unwrap();
        Ok(root)
    }
}
