use ramfs::{KernelProvider, RamFs};
use spin::mutex::Mutex;
use std::error::Error;
use std::sync::Arc;
use vfscore::fstype::{MountFlags, VfsFsType};
use vfscore::utils::{VfsInodeMode, VfsTimeSpec};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    // create the fstype for ramfs
    let ramfs = Arc::new(RamFs::<_, Mutex<()>>::new(PageProviderImpl));
    // create a real ramfs
    // This function will return the root dentry of the ramfs
    let root = ramfs.clone().mount(MountFlags::empty(), "", &[])?;
    // we can get the super block from the dentry
    let sb = root.get_super_block()?;
    // we can get the fstype from the super block
    let fs = sb.get_fs_type();
    // we can create a raw-file from the fstype
    let root_file = fs.make_vfs_file(root.clone())?;
    // write dir will return a error
    root_file.write_at(0, &[0;10]).is_err().then(||println!("write to dir error"));

    // we can get the inode from the dentry
    let root_inode = root.get_inode()?;
    // call VfsInode interface to create a file
    let test_inode = root_inode.create("test",VfsInodeMode::from_bits_truncate(0o777),sb.clone())?;
    let test_dentry = root.clone().insert("test",test_inode)?;
    let test_file = fs.make_vfs_file(test_dentry)?;
    test_file.write_at(0, &[b'x';10]).is_ok().then(||println!("write to file xxxxxxxxxx ok"));
    let mut buf = [0u8; 10];
    test_file.read_at(0, &mut buf).is_ok().then(||println!("read file ok, the content is {}",core::str::from_utf8(&buf).unwrap()));

    // unmount the ramfs
    ramfs.kill_sb(sb)?;

    Ok(())
}

#[derive(Debug, Clone)]
struct PageProviderImpl;

impl KernelProvider for PageProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0,0)
    }
}
