use fat_vfs::{FatFs, FatFsProvider};
use fatfs::{format_volume, FormatVolumeOptions, IoBase, Read, Seek, SeekFrom, Write};
use spin::Mutex;
use std::error::Error;
use std::fs::{File, OpenOptions};

use fatfs::FatType::Fat32;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::sync::Arc;
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::fstype::{MountFlags, VfsFsType};
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{FileStat, VfsNodeType, VfsTimeSpec};
use vfscore::VfsResult;

#[derive(Clone)]
struct ProviderImpl;
impl FatFsProvider for ProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("fat32.img")
        .unwrap();
    file.set_len(64 * 1024 * 1024).unwrap();
    let file = Arc::new(Mutex::new(file));

    {
        let mut buf_file = BufStream::new(file.clone());
        format_volume(&mut buf_file, FormatVolumeOptions::new().fat_type(Fat32)).unwrap();
        let fs = fatfs::FileSystem::new(buf_file, fatfs::FsOptions::new()).unwrap();
        let root_dir = fs.root_dir();
        let _file = root_dir.create_file("root.txt").unwrap();

        root_dir.iter().for_each(|x| {
            let name = x.unwrap().file_name();
            println!("{:?}", name);
        });

        fs.unmount().unwrap();
    }

    let fatfs = Arc::new(FatFs::<_, Mutex<()>>::new(ProviderImpl));
    let root = fatfs.clone().mount(
        MountFlags::empty(),
        Some(Arc::new(DeviceInode::new(file.clone()))),
        &[],
    )?;
    assert_eq!(fatfs.fs_name(), "fatfs");

    let _d1 = root
        .inode()?
        .create("d1", VfsNodeType::Dir, "rwxrwxrwx".into(), None)?;
    let _f1 = root
        .inode()?
        .create("f1", VfsNodeType::File, "rwxrwxrwx".into(), None)?;

    println!("root dir: ");
    // readdir
    let mut index = 0;
    loop {
        let dir_entry = root.inode()?.readdir(index)?;
        if dir_entry.is_none() {
            break;
        }
        let dir_entry = dir_entry.unwrap();
        println!("{:?}", dir_entry);
        index += 1;
    }

    let sb = root.inode()?.get_super_block()?;
    fatfs.kill_sb(sb)?;

    {
        // reset file
        use std::io::Seek;
        file.lock().seek(std::io::SeekFrom::Start(0)).unwrap();
        let buf_file = BufStream::new(file.clone());
        let fs = fatfs::FileSystem::new(buf_file, fatfs::FsOptions::new()).unwrap();
        let root_dir = fs.root_dir();
        let _file = root_dir.create_file("root.txt").unwrap();

        root_dir.iter().for_each(|x| {
            let name = x.unwrap().file_name();
            println!("{:?}", name);
        });
    }

    std::fs::remove_file("fat32.img").unwrap();
    Ok(())
}

struct BufStream {
    file: Arc<Mutex<File>>,
}

impl BufStream {
    pub fn new(file: Arc<Mutex<File>>) -> Self {
        BufStream { file }
    }
}

impl IoBase for BufStream {
    type Error = ();
}

impl Read for BufStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        use std::io::Read;
        self.file.lock().read(buf).map_err(|_| ())
    }
}

impl Write for BufStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        use std::io::Write;
        self.file.lock().write(buf).map_err(|_| ())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        use std::io::Write;
        self.file.lock().flush().map_err(|_| ())
    }
}

impl Seek for BufStream {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        use std::io::Seek;
        match pos {
            SeekFrom::Start(pos) => self
                .file
                .lock()
                .seek(std::io::SeekFrom::Start(pos))
                .map_err(|_| ()),
            SeekFrom::End(pos) => self
                .file
                .lock()
                .seek(std::io::SeekFrom::End(pos))
                .map_err(|_| ()),
            SeekFrom::Current(pos) => self
                .file
                .lock()
                .seek(std::io::SeekFrom::Current(pos))
                .map_err(|_| ()),
        }
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
        self.file
            .lock()
            .read_at(buf, offset)
            .map_err(|_| VfsError::IoError)
    }
    fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        self.file
            .lock()
            .write_at(buf, offset)
            .map_err(|_| VfsError::IoError)
    }
    fn flush(&self) -> VfsResult<()> {
        self.fsync()
    }
    fn fsync(&self) -> VfsResult<()> {
        self.file.lock().sync_all().map_err(|_| VfsError::IoError)
    }
}

impl VfsInode for DeviceInode {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        todo!()
    }

    fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        todo!()
    }

    fn get_attr(&self) -> VfsResult<FileStat> {
        let meta = self.file.lock().metadata().unwrap();
        Ok(FileStat {
            st_dev: meta.dev(),
            st_ino: meta.ino(),
            st_mode: meta.mode(),
            st_nlink: meta.nlink() as u32,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size: meta.size(),
            st_blksize: meta.blksize() as u32,
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
