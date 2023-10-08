use devfs::{DevFs, DevKernelProvider};
use log::error;
use spin::Mutex;
use std::error::Error;
use std::sync::Arc;
use vfscore::error::VfsError;
use vfscore::file::VfsFile;
use vfscore::fstype::{MountFlags, VfsFsType};
use vfscore::inode::{InodeAttr, VfsInode};
use vfscore::superblock::VfsSuperBlock;
use vfscore::utils::{FileStat, VfsNodePerm, VfsNodeType, VfsTimeSpec};
use vfscore::VfsResult;

#[derive(Clone)]
struct DevFsKernelProviderImpl;

impl DevKernelProvider for DevFsKernelProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
    fn rdev2device(&self, rdev: u32) -> Option<Arc<dyn VfsInode>> {
        match rdev {
            0 => Some(Arc::new(NullDev)),
            1 => Some(Arc::new(NullDev)),
            _ => None,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let devfs = Arc::new(DevFs::<_, Mutex<()>>::new(DevFsKernelProviderImpl));
    let root_dt = devfs.clone().mount(MountFlags::empty(), "", &[])?;
    let root_inode = root_dt.inode()?;
    root_inode.create(
        "null",
        VfsNodeType::CharDevice,
        VfsNodePerm::from_bits_truncate(0o666),
        Some(0),
    )?;
    root_inode.create(
        "zero",
        VfsNodeType::CharDevice,
        VfsNodePerm::from_bits_truncate(0o666),
        Some(1),
    )?;
    root_inode
        .create(
            "test",
            VfsNodeType::File,
            VfsNodePerm::from_bits_truncate(0o666),
            None,
        )
        .is_err()
        .then(|| error!("should not create file"));
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
    let null_inode = root_inode.lookup("null")?.unwrap();
    let zero_inode = root_inode.lookup("zero")?.unwrap();

    let w = null_inode.write_at(0, &[0u8; 10])?;
    assert_eq!(w, 10);
    let w = zero_inode.write_at(0, &[0u8; 10])?;
    assert_eq!(w, 10);
    let mut buf = [1; 10];
    let r = null_inode.read_at(0, &mut buf)?;
    assert_eq!(r, 10);
    assert_eq!(buf, [0; 10]);

    assert_eq!(null_inode.inode_type(), VfsNodeType::CharDevice);

    let stat = null_inode.get_attr()?;
    println!("{:#?}", stat);

    let sb = null_inode.get_super_block()?;
    devfs.kill_sb(sb)?;
    Ok(())
}

struct NullDev;

impl VfsFile for NullDev {
    fn read_at(&self, _offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        buf.fill(0);
        Ok(buf.len())
    }
    fn write_at(&self, _offset: u64, buf: &[u8]) -> VfsResult<usize> {
        Ok(buf.len())
    }
}

impl VfsInode for NullDev {
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        todo!()
    }

    fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        Ok(())
    }

    fn get_attr(&self) -> VfsResult<FileStat> {
        Err(VfsError::NoSys)
    }

    fn inode_type(&self) -> VfsNodeType {
        todo!()
    }
}
