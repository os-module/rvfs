#![no_std]
#![feature(trait_alias)]
#![feature(iter_advance_by)]

mod blk;
mod inode;
mod types;

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
};

pub use inode::special::ExtDevProvider;
use lock_api::Mutex;
use log::info;
pub use lwext4_rs::FsType as ExtFsType;
use lwext4_rs::{BlockDevice, FsType, MountHandle, RegisterHandle};
use unifs::dentry::UniFsDentry;
use vfscore::{
    dentry::VfsDentry,
    error::VfsError,
    fstype::{FileSystemFlags, VfsFsType},
    inode::VfsInode,
    superblock::{SuperType, VfsSuperBlock},
    utils::{VfsFsStat, VfsNodeType},
    VfsResult,
};

use crate::{
    blk::ExtDevice,
    inode::dir::ExtDirInode,
    types::{into_vfs, ToDir},
};
pub trait VfsRawMutex = lock_api::RawMutex + Send + Sync;

type FileSystem = lwext4_rs::FileSystem<ExtDevice>;

pub struct ExtFs<T, R: VfsRawMutex> {
    ty: ExtFsType,
    fs_container: Mutex<R, BTreeMap<usize, Arc<ExtFsSuperBlock<R>>>>,
    provider: Arc<T>,
}

impl<T, R: VfsRawMutex> ExtFs<T, R> {
    pub fn new(ty: ExtFsType, provider: T) -> Self {
        Self {
            ty,
            fs_container: Mutex::new(BTreeMap::new()),
            provider: Arc::new(provider),
        }
    }
}

impl<T: ExtDevProvider + 'static, R: VfsRawMutex + 'static> VfsFsType for ExtFs<T, R> {
    fn mount(
        self: Arc<Self>,
        _flags: u32,
        ab_mnt: &str,
        dev: Option<Arc<dyn VfsInode>>,
        _data: &[u8],
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        let dev = dev.ok_or(VfsError::Invalid)?;
        if dev.inode_type() != VfsNodeType::BlockDevice {
            return Err(VfsError::Invalid);
        }
        let ab_mnt = &ab_mnt.to_dir();
        let dev_ino = dev.get_attr()?.st_rdev;
        // For same device, we only mount once, but we will return different dentry according to ab_mnt(absolute mount point)
        if let Some(sb) = self.fs_container.lock().get(&(dev_ino as usize)) {
            return sb.root_dentry(ab_mnt);
        }
        info!("extfs: mount: mp: {}", ab_mnt);
        let ext_dev = ExtDevice::new(dev)?;
        let sb = ExtFsSuperBlock::<R>::new(
            &(self.clone() as Arc<dyn VfsFsType>),
            ext_dev,
            ab_mnt,
            self.provider.clone(),
        )?;
        // we use dev_ino as the key to store the superblock
        self.fs_container
            .lock()
            .insert(dev_ino as usize, sb.clone());
        sb.root_dentry(ab_mnt)
    }

    fn kill_sb(&self, sb: Arc<dyn VfsSuperBlock>) -> VfsResult<()> {
        if let Ok(sb) = sb.downcast_arc::<ExtFsSuperBlock<R>>() {
            let dev_ino = sb.ext_dev.device_file.get_attr()?.st_rdev;
            let sb = self.fs_container.lock().remove(&(dev_ino as usize));
            if let Some(sb) = sb {
                // todo!(call unmount)
                sb.sync_fs(false)?;
                info!("extfs: kill_sb: remove sb for dev {}", dev_ino);
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

    fn fs_name(&self) -> String {
        match self.ty {
            FsType::Ext2 => "ext2".to_string(),
            FsType::Ext3 => "ext3".to_string(),
            FsType::Ext4 => "ext4".to_string(),
        }
    }
}

struct ExtFsSuperBlock<R: VfsRawMutex> {
    ext_dev: ExtDevice,
    fs_type: Weak<dyn VfsFsType>,
    root: Mutex<R, Option<Arc<dyn VfsInode>>>,
    fs: FileSystem,
    mnt_info: Mutex<R, BTreeMap<String, Arc<dyn VfsDentry>>>,
    provider: Arc<dyn ExtDevProvider>,
}

unsafe impl<R: VfsRawMutex> Send for ExtFsSuperBlock<R> {}
unsafe impl<R: VfsRawMutex> Sync for ExtFsSuperBlock<R> {}

impl<R: VfsRawMutex + 'static> ExtFsSuperBlock<R> {
    fn new(
        fs_type: &Arc<dyn VfsFsType>,
        device: ExtDevice,
        ab_mnt: &str,
        provider: Arc<dyn ExtDevProvider>,
    ) -> VfsResult<Arc<Self>> {
        let blk = BlockDevice::new(device.clone());
        let register_handler =
            RegisterHandle::register(blk, "ext4fs".to_string()).map_err(into_vfs)?;
        info!("register ext fs");
        let mount_handler = MountHandle::mount(register_handler, ab_mnt.to_string(), true, false)
            .map_err(into_vfs)?;
        let fs = FileSystem::new(mount_handler).map_err(into_vfs)?;
        info!("create ext fs");
        let dir = fs.readdir(ab_mnt).map_err(into_vfs)?;

        let sb = Arc::new(Self {
            ext_dev: device,
            fs_type: Arc::downgrade(fs_type),
            root: Mutex::new(None),
            fs,
            mnt_info: Mutex::new(BTreeMap::new()),
            provider,
        });

        let dir = Arc::new(Mutex::new(dir));
        let dir = ExtDirInode::new(dir, &sb);
        let root_inode = Arc::new(dir);

        sb.root.lock().replace(root_inode.clone());
        let parent = Weak::<UniFsDentry<R>>::new();
        let root_dt = Arc::new(UniFsDentry::<R>::root(root_inode, parent));
        sb.mnt_info.lock().insert(ab_mnt.into(), root_dt.clone());
        Ok(sb)
    }
    pub fn root_dentry(&self, ab_mnt: &str) -> VfsResult<Arc<dyn VfsDentry>> {
        self.mnt_info.lock().get(ab_mnt).map_or_else(
            || {
                let parent = Weak::<UniFsDentry<R>>::new();
                let inode = self.root.lock().clone().unwrap();
                let new = Arc::new(UniFsDentry::<R>::root(inode, parent));
                self.mnt_info.lock().insert(ab_mnt.into(), new.clone());
                Ok(new as Arc<dyn VfsDentry>)
            },
            |x| Ok(x.clone()),
        )
    }
}

impl<R: VfsRawMutex + 'static> VfsSuperBlock for ExtFsSuperBlock<R> {
    fn sync_fs(&self, _wait: bool) -> VfsResult<()> {
        self.ext_dev.device_file.flush()?;
        self.ext_dev.device_file.fsync()?;
        Ok(())
    }

    fn stat_fs(&self) -> VfsResult<VfsFsStat> {
        let mp = self.fs.mount_handle();
        let stat = mp.stats().map_err(into_vfs)?;
        Ok(VfsFsStat {
            f_type: 0x777,
            f_bsize: stat.block_size as i64,
            f_blocks: stat.blocks_count,
            f_bfree: stat.free_blocks_count,
            f_bavail: stat.free_blocks_count,
            f_files: stat.inodes_count as u64,
            f_ffree: stat.inodes_count as u64,
            f_fsid: [0, 0],
            f_namelen: 255,
            f_frsize: stat.block_size as isize * stat.blocks_per_group as isize,
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
    fn root_inode(&self) -> VfsResult<Arc<dyn VfsInode>> {
        self.root.lock().clone().ok_or(VfsError::Invalid)
    }
}
