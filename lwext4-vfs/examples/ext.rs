#![feature(seek_stream_len)]

use log::info;
use lwext4_rs::{
    BlockDeviceConfig, DefaultInterface, FileSystem, FsBuilder, FsType, MountHandle, RegisterHandle,
};
use lwext4_vfs::{ExtDevProvider, ExtFs, ExtFsType};
use spin::Mutex;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::Seek;
use std::sync::Arc;
use embedded_io::{Read, SeekFrom, Write};
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::fstype::VfsFsType;
use vfscore::inode::VfsInode;
use vfscore::path::{DirIter, VfsPath};
use vfscore::utils::{VfsFileStat, VfsNodePerm, VfsNodeType, VfsTime, VfsTimeSpec};
use vfscore::VfsResult;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    mkfs();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("./ext_image")
        .unwrap();
    let extfs = Arc::new(ExtFs::<ProviderImpl, Mutex<()>>::new(
        ExtFsType::Ext3,
        ProviderImpl,
    ));
    let root = extfs
        .clone()
        .mount(
            0,
            "/",
            Some(Arc::new(DeviceInode::new(Arc::new(Mutex::new(file))))),
            &[],
        )
        .unwrap();
    info!("ramfs tree:");
    // print_fs_tree(&mut OutPut, root.clone(), "".to_string(), true).unwrap();
    let path = VfsPath::new(root.clone());
    let dir1_path = path.join("dir").unwrap();
    let dir1 = dir1_path.open(None)?;
    dir1.inode()?.children().for_each(|e| {
        println!("{:?}", e);
    });
    dir1.inode()?.unlink("f1")?;
    dir1.inode()?.symlink("f1", "/dir/f2")?;
    let f4 = dir1.inode()?.create("f4",VfsNodeType::File,VfsNodePerm::from_bits_truncate(0o777),None)?;
    dir1.inode()?.link("f5", f4)?;
    dir1.inode()?.children().for_each(|e| {
        println!("{:?}", e);
    });

    let f3_path = path.join("f3").unwrap();
    let f3 = f3_path.open(None)?;
    let mut buf = [0u8; 512];
    let r = f3.inode()?.read_at(0,&mut buf)?;
    assert_eq!(r, 11);
    assert_eq!(&buf[..r], b"hello world");
    println!("read: {}", std::str::from_utf8(&buf[..r]).unwrap());
    let data = [0x55u8;512];
    let w = f3.inode()?.write_at(0,&data)?;
    assert_eq!(w, 512);
    f3.inode()?.update_time(VfsTime::AccessTime(VfsTimeSpec::new(1,1)),VfsTimeSpec::new(1,1))?;
    let attr = f3.inode()?.get_attr()?;
    println!("{:#x?}", attr);

    let f6 = dir1.inode()?.create("f6",VfsNodeType::File,VfsNodePerm::from_bits_truncate(0o777),None)?;
    let buf = [0u8; 1024];
    let w = f6.write_at(10, &buf)?;
    assert_eq!(w, 1024);
    let attr = f6.get_attr()?;
    assert_eq!(attr.st_size, 1034);

    let mut buf = vec![0u8; 1034];
    let r = f6.read_at(0, &mut buf)?;
    assert_eq!(r, 1034);

    // delete file
    std::fs::remove_file("./ext_image")?;
    Ok(())
}

struct ProviderImpl;
impl ExtDevProvider for ProviderImpl {
    fn rdev2device(&self, _rdev: u64) -> Option<Arc<dyn VfsInode>> {
        None
    }
}

struct OutPut;
impl core::fmt::Write for OutPut {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        print!("{}", s);
        Ok(())
    }
}

fn mkfs() {
    use embedded_io::Seek;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("./ext_image")
        .unwrap();
    file.set_len(1024 * 1024 * 2).unwrap();
    let mut config = BlockDeviceConfig::default();
    let bs: u64 = 512;
    config.block_size = bs as u32;
    config.part_size = file.metadata().unwrap().len();
    config.part_offset = 0;
    config.block_count = config.part_size / bs;

    println!("config: {:#x?}", config);
    // set_debug_mask(DebugFlags::ALL);

    let blk = DefaultInterface::new_device(file, config);
    let fs = FsBuilder::new()
        .ty(FsType::Ext4)
        .journal(true)
        .block_size(1024)
        .label("ext4fs")
        .build(blk)
        .unwrap();
    println!("{:#x?}", fs.fs_info());
    let blk = fs.take_device();
    let register_handler = RegisterHandle::register(blk, "ext4fs".to_string()).unwrap();
    let mount_handler = MountHandle::mount(register_handler, "/".to_string(), true, false).unwrap();
    let fs = FileSystem::new(mount_handler).unwrap();
    fs.create_dir("/dir").unwrap();
    fs.create_dir("/dir2").unwrap();
    fs.file_builder()
        .write(true)
        .create(true)
        .open("/dir/f1")
        .unwrap();
    fs.file_builder()
        .write(true)
        .create(true)
        .open("/dir/f2")
        .unwrap();
    let mut file = fs.file_builder()
        .read(true)
        .write(true)
        .create(true)
        .open("/f3")
        .unwrap();
    file.write_all(b"hello world").unwrap();
    let mut buf = [0u8; 512];
    file.seek(SeekFrom::Start(0)).unwrap();

    let r = file.read(&mut buf).unwrap();
    assert_eq!(r, 11);
    assert_eq!(&buf[..r], b"hello world");

    let dir = fs.readdir("/").unwrap();
    for entry in dir {
        println!("{:?}", entry);
    }
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
