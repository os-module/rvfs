use std::{error::Error, sync::Arc};

use devfs::{DevFs, DevKernelProvider};
use log::error;
use spin::Mutex;
use vfscore::{
    file::VfsFile,
    fstype::VfsFsType,
    inode::{InodeAttr, VfsInode},
    path::DirIter,
    utils::*,
    VfsResult,
};

#[derive(Clone)]
struct DevFsKernelProviderImpl;

impl DevKernelProvider for DevFsKernelProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
    fn rdev2device(&self, rdev: u64) -> Option<Arc<dyn VfsInode>> {
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
    let root_dt = devfs.clone().mount(0, "/dev", None, &[])?;
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
    root_inode.children().for_each(|x| println!("{}", x.name));

    let null_inode = root_inode.lookup("null")?;
    let zero_inode = root_inode.lookup("zero")?;

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
    fn node_perm(&self) -> VfsNodePerm {
        VfsNodePerm::empty()
    }

    fn set_attr(&self, _attr: InodeAttr) -> VfsResult<()> {
        Ok(())
    }

    fn get_attr(&self) -> VfsResult<VfsFileStat> {
        Ok(VfsFileStat {
            st_blksize: 512,
            st_rdev: 0,
            st_size: 0,
            ..Default::default()
        })
    }
    fn inode_type(&self) -> VfsNodeType {
        todo!()
    }
}
