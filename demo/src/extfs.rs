use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{Seek};

use std::sync::Arc;
use log::info;
use lwext4_rs::{BlockDeviceConfig, DefaultInterface, FsBuilder, FsType};
use spin::Mutex;
use lwext4_vfs::ExtDevProvider;

use vfscore::dentry::VfsDentry;
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::fstype::VfsFsType;
use vfscore::inode::VfsInode;
use vfscore::utils::{VfsFileStat, VfsNodePerm, VfsNodeType, VfsTimeSpec};
use vfscore::VfsResult;

pub struct ExtFsProviderImpl;
impl ExtDevProvider for ExtFsProviderImpl {
    fn rdev2device(&self, _rdev: u64) -> Option<Arc<dyn VfsInode>> {
        None
    }
}


pub fn init_extfs(extfs: Arc<dyn VfsFsType>) -> Result<Arc<dyn VfsDentry>, Box<dyn Error>>{
    mkfs();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/tmp/ext_image")
        .unwrap();
    let root_dt = extfs.i_mount(0, "/", Some(Arc::new(DeviceInode::new(Arc::new(Mutex::new(file))))), &[])?;
    let root_inode = root_dt.inode()?;
    let _f1 = root_inode.create(
        "f1.txt",
        VfsNodeType::File,
        VfsNodePerm::from_bits_truncate(0o666),
        None,
    )?;
    let _f2 = root_inode.create(
        "f2.txt",
        VfsNodeType::File,
        VfsNodePerm::from_bits_truncate(0o666),
        None,
    )?;
    let _d1 = root_inode.create(
        "d1",
        VfsNodeType::Dir,
        VfsNodePerm::from_bits_truncate(0o755),
        None,
    )?;
    let _d2 = root_inode.create(
        "d2",
        VfsNodeType::Dir,
        VfsNodePerm::from_bits_truncate(0o755),
        None,
    )?;
    info!("init extfs");
    info!("extfs tree:");
    info!(
        r"
    /
    ├── .
    ├── ..
    ├── d1
    ├── d2
    ├── f1.txt
    └── f2.txt
        ");
    Ok(root_dt)
}


fn mkfs(){
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("/tmp/ext_image")
        .unwrap();
    file.set_len(1024 * 1024 * 2).unwrap();
    let mut config = BlockDeviceConfig::default();
    let bs: u64 = 512;
    config.block_size = bs as u32;
    config.part_size = file.metadata().unwrap().len();
    config.part_offset = 0;
    config.block_count = config.part_size / bs;

    let blk = DefaultInterface::new_device(file, config);
    let fs = FsBuilder::new()
        .ty(FsType::Ext4)
        .journal(true)
        .block_size(1024)
        .label("ext4fs")
        .build(blk)
        .unwrap();
    println!("{:#x?}", fs.fs_info());
}


struct DeviceInode {
    file: Arc<Mutex<File>>,
}

impl DeviceInode {
    pub fn new(file: Arc<Mutex<File>>) -> Self {
        DeviceInode { file }
    }
}

impl VfsFile for DeviceInode {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        use std::io::Read;
        self.file
            .lock()
            .seek(std::io::SeekFrom::Start(offset))
            .map_err(|_| VfsError::IoError)?;
        self.file.lock().read(buf).map_err(|_| VfsError::IoError)
    }
    fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        use std::io::Write;
        self.file
            .lock()
            .seek(std::io::SeekFrom::Start(offset))
            .map_err(|_| VfsError::IoError)?;
        self.file.lock().write(buf).map_err(|_| VfsError::IoError)
    }
    fn flush(&self) -> VfsResult<()> {
        self.fsync()
    }
    fn fsync(&self) -> VfsResult<()> {
        use std::io::Write;
        self.file.lock().flush().map_err(|_| VfsError::IoError)
    }
}

impl VfsInode for DeviceInode {
    fn node_perm(&self) -> VfsNodePerm {
        VfsNodePerm::empty()
    }
    fn get_attr(&self) -> VfsResult<VfsFileStat> {
        let mut meta = self.file.lock();

        Ok(VfsFileStat {
            st_dev: 0,
            st_ino: 0,
            st_mode: 0,
            st_nlink: 1,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size: meta.stream_len().unwrap(),
            st_blksize: 512,
            __pad2: 0,
            st_blocks: 0,
            st_atime: VfsTimeSpec::new(0, 0),
            st_mtime: VfsTimeSpec::new(0, 0),
            st_ctime: VfsTimeSpec::new(0, 0),
            unused: 0,
        })
    }
    fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::BlockDevice
    }
}
