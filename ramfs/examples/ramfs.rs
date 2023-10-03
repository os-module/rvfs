use ramfs::{KernelProvider, RamFs};
use spin::mutex::Mutex;
use std::error::Error;
use std::sync::Arc;
use vfscore::fstype::{MountFlags, VfsFsType};
use vfscore::utils::{VfsNodePerm, VfsNodeType, VfsTimeSpec};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    // create the fstype for ramfs
    let ramfs = Arc::new(RamFs::<_, Mutex<()>>::new(PageProviderImpl));
    // create a real ramfs
    // This function will return the root dentry of the ramfs
    let root = ramfs.clone().mount(MountFlags::empty(), "", &[])?;
    // we can get the super block from the inode
    let sb = root.get_inode()?.get_super_block()?;
    // we can get the fstype from the super block
    let fs = sb.get_fs_type();
    // we can create a raw-file from the fstype
    let root_file = fs.make_vfs_file(root.clone())?;



    // write dir will cause a error
    root_file
        .write_at(0, &[0; 10])
        .is_err()
        .then(|| println!("write to dir error"));

    // we can get the inode from the dentry
    let root_inode = root.get_inode()?;
    // call VfsInode interface to create a file
    let test_inode =
        root_inode.create("test", VfsNodeType::File,VfsNodePerm::from_bits_truncate(0o777),None)?;
    let test_dentry = root.clone().insert("test", test_inode)?;
    let test_file = fs.make_vfs_file(test_dentry)?;
    test_file
        .write_at(0, &[b'x'; 10])
        .is_ok()
        .then(|| println!("write to file xxxxxxxxxx ok"));
    let mut buf = [0u8; 10];
    test_file.read_at(0, &mut buf).is_ok().then(|| {
        println!(
            "read file ok, the content is {}",
            core::str::from_utf8(&buf).unwrap()
        )
    });

    // create a mount point
    let mount_dir = root_inode.create(
        "mount_dir",
        VfsNodeType::Dir,
        VfsNodePerm::from_bits_truncate(0o777),
        None,
    )?;
    let mnt_dt = root.clone().insert("mount_dir", mount_dir)?;

    // create a new ramfs
    let new_ramfs_root = ramfs.clone().mount(MountFlags::empty(), "", &[])?;
    let new_sb = new_ramfs_root.get_inode()?.get_super_block()?;
    // mount the ramfs to the mount_dir
    mnt_dt.clone().to_mount_point(new_ramfs_root.clone(), MountFlags::empty())?;
    mnt_dt.is_mount_point().then(|| println!("create a mount point"));

    println!("root dir: ");
    // readdir
    loop {
        let dir_entry= root_file.readdir()?;
        if dir_entry.is_none() {
            break;
        }
        let dir_entry = dir_entry.unwrap();
        println!("{:?}", dir_entry);
    }

    // unmount the ramfs
    ramfs.kill_sb(sb)?;
    ramfs.kill_sb(new_sb)?;

    Ok(())
}

#[derive(Debug, Clone)]
struct PageProviderImpl;

impl KernelProvider for PageProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}
