#![allow(clippy::uninit_vec)]
#![feature(seek_stream_len)]
use fat_vfs::{FatFs, FatFsProvider};
use fatfs::FatType::Fat32;
use fatfs::{format_volume, FormatVolumeOptions, IoBase, Read, Seek, SeekFrom, Write};
use log::info;
use spin::Mutex;
use std::error::Error;
use std::io::Cursor;
use std::sync::Arc;
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::fstype::VfsFsType;
use vfscore::inode::VfsInode;
use vfscore::path::print_fs_tree;

use vfscore::utils::*;
use vfscore::VfsResult;

#[derive(Clone)]
struct ProviderImpl;
impl FatFsProvider for ProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}

struct FakeWriter;

impl core::fmt::Write for FakeWriter {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        print!("{}", s);
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let mut data = Vec::with_capacity(64 * 1024 * 1024);
    unsafe {
        data.set_len(64 * 1024 * 1024);
        data.fill(0);
    }
    let mut file = Cursor::new(data);
    use std::io::Seek;
    info!(
        "mem file len: {:?} MB",
        file.stream_len().unwrap() / 1024 / 1024
    );
    let file = Arc::new(Mutex::new(file));

    {
        let mut buf_file = BufStream::new(file.clone());
        format_volume(&mut buf_file, FormatVolumeOptions::new().fat_type(Fat32)).unwrap();
        let fs = fatfs::FileSystem::new(buf_file, fatfs::FsOptions::new()).unwrap();
        let root_dir = fs.root_dir();
        let file = root_dir.create_file("root.txt").unwrap();
        // /
        // |-- root.txt
        file.get_fs().unmount().unwrap();
        // fs.unmount().unwrap();
    }

    let fatfs = Arc::new(FatFs::<_, Mutex<()>>::new(ProviderImpl));
    let root = fatfs
        .clone()
        .mount(0, "/", Some(Arc::new(DeviceInode::new(file.clone()))), &[])?;
    assert_eq!(fatfs.fs_name(), "fatfs");

    let _d1 = root
        .inode()?
        .create("d1", VfsNodeType::Dir, "rwxrwxrwx".into(), None)?;
    let f1 = root
        .inode()?
        .create("f1", VfsNodeType::File, "rwxrwxrwx".into(), None)?;

    let f2 = root
        .inode()?
        .create("f2", VfsNodeType::File, "rwxrwxrwx".into(), None)?;

    let f3 = root
        .inode()?
        .create("f3", VfsNodeType::File, "rwxrwxrwx".into(), None)?;

    let mut offset = 0;
    let mut buf = [0u8; 1024];
    let mut data = 1;
    loop {
        buf.fill(data);
        let w = f1.write_at(offset, &buf)?;
        assert_eq!(w, 1024);
        offset += w as u64;
        data = (data + 1) % 255;
        if offset >= 1024 * 1024 * 60 {
            break;
        } // 60MB
    }
    f1.flush()?;
    println!("write 60MB to f1");
    root.inode()?.unlink("f1")?;
    println!("unlink f1");
    offset = 0;
    data = 1;
    loop {
        buf.fill(data);
        let w = f2
            .write_at(offset, &buf)
            .map_err(|e| match e {
                VfsError::NoSpace => println!("disk no space, offset: {}MB", offset / 1024 / 1024),
                e => println!("error: {:?}", e),
            })
            .unwrap();
        assert_eq!(w, 1024);
        offset += w as u64;
        data = (data + 1) % 255;
        if offset >= 1024 * 1024 * 60 {
            break;
        } // 60MB
    }

    println!("write 60MB data to f2");
    buf.fill(0);
    let r = f2.read_at(1024, &mut buf)?;
    assert_eq!(r, 1024);
    assert_eq!(buf, [2u8; 1024]);
    f2.flush()?;

    println!("read 1024 bytes from f2");

    f3.truncate(10)?;
    let stat = f3.get_attr()?;
    assert_eq!(stat.st_size, 10);
    let w = f3.write_at(10, &[1u8; 10])?;
    assert_eq!(w, 10);
    f3.flush()?;
    let stat = f3.get_attr()?;
    assert_eq!(stat.st_size, 20);
    println!("truncate file success");

    println!("root dir: ");
    // /
    // |-- root.txt
    // |--d1
    //    |--.
    //    |--..
    // |--f2
    // |--f3
    print_fs_tree(&mut FakeWriter, root.clone(), "".to_string())?;
    let sb = root.inode()?.get_super_block()?;
    fatfs.kill_sb(sb)?; //like unmount up

    {
        // reset file
        file.lock().seek(std::io::SeekFrom::Start(0)).unwrap();
        let buf_file = BufStream::new(file.clone());
        let fs = fatfs::FileSystem::new(buf_file, fatfs::FsOptions::new()).unwrap();
        let root_dir = fs.root_dir();
        root_dir.iter().for_each(|x| {
            let name = x.unwrap().file_name();
            println!("{:?}", name);
        });
    }
    Ok(())
}

struct BufStream {
    file: Arc<Mutex<Cursor<Vec<u8>>>>,
}

impl BufStream {
    pub fn new(file: Arc<Mutex<Cursor<Vec<u8>>>>) -> Self {
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
    file: Arc<Mutex<Cursor<Vec<u8>>>>,
}

impl DeviceInode {
    pub fn new(file: Arc<Mutex<Cursor<Vec<u8>>>>) -> Self {
        DeviceInode { file }
    }
}

impl VfsFile for DeviceInode {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        use std::io::{Read, Seek};
        self.file
            .lock()
            .seek(std::io::SeekFrom::Start(offset))
            .map_err(|_| VfsError::IoError)?;
        self.file.lock().read(buf).map_err(|_| VfsError::IoError)
    }
    fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        use std::io::{Seek, Write};
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
        use std::io::Seek;
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
