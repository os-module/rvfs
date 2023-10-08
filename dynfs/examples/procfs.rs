use dynfs::{DynFs, DynFsDirInode, DynFsKernelProvider};

use spin::Mutex;
use std::cmp::min;
use std::error::Error;
use std::sync::Arc;
use vfscore::file::VfsFile;
use vfscore::fstype::{MountFlags, VfsFsType};
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{FileStat, VfsNodeType, VfsTimeSpec};
use vfscore::VfsResult;

#[derive(Clone)]
struct DynFsKernelProviderImpl;

impl DynFsKernelProvider for DynFsKernelProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let procfs = Arc::new(DynFs::<_, Mutex<()>>::new(
        DynFsKernelProviderImpl,
        "procfs",
    ));
    let root_dt = procfs.clone().mount(MountFlags::empty(), None, &[])?;
    let root_inode = root_dt.inode()?;

    // Procfs don't support to create file/dir at runtime
    assert!(root_inode
        .create("1", VfsNodeType::File, "r--r--r--".into(), None)
        .is_err());
    assert!(root_inode
        .create("1", VfsNodeType::Dir, "r--r--r--".into(), None,)
        .is_err());

    type DynFsDirInodeImpl = DynFsDirInode<DynFsKernelProviderImpl, Mutex<()>>;

    let dynfs_inode = root_inode
        .clone()
        .downcast_arc::<DynFsDirInodeImpl>()
        .unwrap();

    // procfs support add file manually
    dynfs_inode.add_file_manually("2", Arc::new(ProcessInfo), "r--r--r--".into())?;
    dynfs_inode.add_dir_manually("3", "r-xr-xr-x".into())?;

    println!("root dir: ");
    // readdir
    let mut index = 0;
    loop {
        let dir_entry = root_inode.readdir(index)?;
        if dir_entry.is_none() {
            break;
        }
        let dir_entry = dir_entry.unwrap();
        println!("{:?}", dir_entry);
        index += 1;
    }

    let p = root_inode.lookup("2")?.unwrap();
    let mut buf = [0; 10];
    let r = p.read_at(0, &mut buf)?;
    let content = core::str::from_utf8(&buf[..r]).unwrap();
    println!("The content is:\n{content}");

    // Procfs support remove file manually
    dynfs_inode.remove_manually("2")?;
    dynfs_inode.remove_manually("3")?;
    println!("root dir: ");
    // readdir
    let mut index = 0;
    loop {
        let dir_entry = root_inode.readdir(index)?;
        if dir_entry.is_none() {
            break;
        }
        let dir_entry = dir_entry.unwrap();
        println!("{:?}", dir_entry);
        index += 1;
    }

    procfs.kill_sb(root_inode.get_super_block()?)?;

    Ok(())
}

struct ProcessInfo;

impl VfsFile for ProcessInfo {
    fn read_at(&self, _offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let data = b"pid:2";
        let min_len = min(data.len(), buf.len());
        buf[..min_len].copy_from_slice(&data[..min_len]);
        Ok(min_len)
    }
}

impl VfsInode for ProcessInfo {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        todo!()
    }

    fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        todo!()
    }

    fn get_attr(&self) -> VfsResult<FileStat> {
        todo!()
    }

    fn inode_type(&self) -> VfsNodeType {
        todo!()
    }
}
