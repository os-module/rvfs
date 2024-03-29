#![feature(seek_stream_len)]

use std::{collections::HashMap, error::Error, ops::Index, sync::Arc};

use ::devfs::DevFs;
use ::ramfs::RamFs;
use dynfs::DynFs;
use log::info;
use lwext4_vfs::{ExtFs, ExtFsType};
use spin::{Lazy, Mutex};
use vfscore::{
    dentry::VfsDentry,
    error::VfsError,
    fstype::VfsFsType,
    path::{print_fs_tree, VfsPath},
    utils::{VfsInodeMode, VfsNodeType},
};

use crate::{
    devfs::{init_devfs, DevFsKernelProviderImpl},
    extfs::{init_extfs, ExtFsProviderImpl},
    procfs::{init_procfs, DynFsKernelProviderImpl, ProcFsDirInodeImpl, ProcessInfo},
    ramfs::{init_ramfs, RamFsProviderImpl},
};

mod devfs;
mod extfs;
mod procfs;
mod ramfs;

static FS: Lazy<Mutex<HashMap<String, Arc<dyn VfsFsType>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    register_all_fs();
    let ramfs_root = init_ramfs(FS.lock().index("ramfs").clone())?;
    let procfs_root = init_procfs(FS.lock().index("procfs").clone())?;
    let devfs_root = init_devfs(FS.lock().index("devfs").clone())?;
    let extfs_root = init_extfs(FS.lock().index("extfs").clone())?;
    ramfs_root
        .inode()?
        .create("proc", VfsNodeType::Dir, "rwxr-xr-x".into(), None)?;
    ramfs_root
        .inode()?
        .create("dev", VfsNodeType::Dir, "rwxr-xr-x".into(), None)?;
    ramfs_root
        .inode()?
        .create("ext", VfsNodeType::Dir, "rwxr-xr-x".into(), None)?;
    let path = VfsPath::new(ramfs_root.clone(), ramfs_root.clone());
    path.join("proc")?.mount(procfs_root, 0)?;
    path.join("dev")?.mount(devfs_root, 0)?;
    path.join("ext")?.mount(extfs_root, 0)?;
    let test1_path = path.join("/d1/test1.txt")?;

    let dt1 = test1_path.open(Some(
        VfsInodeMode::from_bits_truncate(0o777) | VfsInodeMode::FILE,
    ))?;
    let test2_path = path.join("/d1/test2.txt")?;
    let dt2 = test2_path.open(Some(
        VfsInodeMode::from_bits_truncate(0o777) | VfsInodeMode::FILE,
    ))?;

    dt1.inode()?.write_at(0, b"hello world")?;
    dt2.inode()?.write_at(0, b"test2")?;

    let proc_path = path.join("proc")?;
    let proc_dt = proc_path.open(None)?;

    let proc_inode = proc_dt
        .inode()?
        .downcast_arc::<ProcFsDirInodeImpl>()
        .map_err(|_| VfsError::Invalid)?;
    let pid1 = proc_inode.add_dir_manually("1", "r-xr-xr-x".into())?;
    let pid1_dt = proc_dt
        .i_insert("1", pid1.clone())
        .map_err(|_| VfsError::Invalid)?;
    let pid1 = pid1
        .downcast_arc::<ProcFsDirInodeImpl>()
        .map_err(|_| VfsError::Invalid)?;
    let pid1pid =
        pid1.add_file_manually("pid", Arc::new(ProcessInfo::new(1)), "r--r--r--".into())?;
    pid1_dt.i_insert("pid", pid1pid)?;

    open_symlink_test(ramfs_root.clone())?;

    info!("ramfs tree:");
    print_fs_tree(&mut OutPut, ramfs_root.clone(), "".to_string(), true)?;
    Ok(())
}

fn open_symlink_test(root: Arc<dyn VfsDentry>) -> Result<(), Box<dyn Error>> {
    let path = VfsPath::new(root.clone(), root);
    path.join("f1_link.txt")?.symlink("f1.txt")?;
    path.join("./d1/test1_link")?.symlink("test1.txt")?;

    path.join("/d1/f1_link")?.symlink("/f1.txt")?;
    path.join("/d1/f1_link1")?.symlink("../f1.txt")?;

    let test1 = path.join("/d1/test1_link")?.open(None)?;
    let test1 = test1.inode()?;
    let mut buf = [0u8; 255];
    let r = test1.read_at(0, &mut buf)?;
    println!(
        "read symlink test1.txt: {:?}",
        std::str::from_utf8(&buf[..r])?
    );

    let f1 = path.join("/d1/f1_link")?.open(None)?;
    let f1 = f1.inode()?;
    let r = f1.read_at(0, &mut buf)?;
    println!(
        "read symlink /d1/f1_link: {:?}",
        std::str::from_utf8(&buf[..r])?
    );

    let f11 = path.join("/d1/f1_link1")?.open(None)?;
    let f11 = f11.inode()?;
    let r = f11.read_at(0, &mut buf)?;
    println!(
        "read symlink /d1/f1_link1: {:?}",
        std::str::from_utf8(&buf[..r])?
    );

    let f11 = path
        .join("/d1/f1_link1")?
        .open2(None, pconst::io::OpenFlags::O_NOFOLLOW)?;
    let f11 = f11.inode()?;
    f11.read_at(0, &mut buf)
        .expect_err("read symlink /d1/f1_link1: expect error");
    let r = f11.readlink(&mut buf)?;
    println!(
        "read symlink content /d1/f1_link1: {:?}",
        std::str::from_utf8(&buf[..r])?
    );
    Ok(())
}

struct OutPut;
impl core::fmt::Write for OutPut {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        print!("{}", s);
        Ok(())
    }
}

fn register_all_fs() {
    let procfs = Arc::new(DynFs::<_, Mutex<()>>::new(
        DynFsKernelProviderImpl,
        "procfs",
    ));
    let sysfs = Arc::new(DynFs::<_, Mutex<()>>::new(DynFsKernelProviderImpl, "sysfs"));
    let ramfs = Arc::new(RamFs::<_, Mutex<()>>::new(RamFsProviderImpl));
    let devfs = Arc::new(DevFs::<_, Mutex<()>>::new(DevFsKernelProviderImpl));
    let extfs = Arc::new(ExtFs::<_, Mutex<()>>::new(
        ExtFsType::Ext3,
        ExtFsProviderImpl,
    ));

    FS.lock().insert("procfs".to_string(), procfs);
    FS.lock().insert("sysfs".to_string(), sysfs);
    FS.lock().insert("ramfs".to_string(), ramfs);
    FS.lock().insert("devfs".to_string(), devfs);
    FS.lock().insert("extfs".to_string(), extfs);
    info!("register all fs");
}
