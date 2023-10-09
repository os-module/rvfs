use crate::devfs::{init_devfs, DevFsKernelProviderImpl};
use ::devfs::DevFs;
use ::ramfs::RamFs;
use dynfs::DynFs;
use log::info;
use spin::{Lazy, Mutex};
use std::collections::HashMap;
use std::error::Error;
use std::ops::Index;
use std::sync::Arc;
use vfscore::fstype::{MountFlags, VfsFsType};
use vfscore::path::VfsPath;
use vfscore::utils::VfsNodeType;

use crate::procfs::{init_procfs, DynFsKernelProviderImpl, ProcFsDirInodeImpl, ProcessInfo};
use crate::ramfs::{init_ramfs, RamFsProviderImpl};
use crate::utils::print_fs_tree;

mod devfs;
mod procfs;
mod ramfs;
mod utils;

static FS: Lazy<Mutex<HashMap<String, Arc<dyn VfsFsType>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    register_all_fs();
    let ramfs_root = init_ramfs(FS.lock().index("ramfs").clone())?;
    let procfs_root = init_procfs(FS.lock().index("procfs").clone())?;
    let devfs_root = init_devfs(FS.lock().index("devfs").clone())?;

    let proc_inode =
        ramfs_root
            .inode()?
            .create("proc", VfsNodeType::Dir, "rwxr-xr-x".into(), None)?;
    let dev_inode =
        ramfs_root
            .inode()?
            .create("dev", VfsNodeType::Dir, "rwxr-xr-x".into(), None)?;
    let proc_dt = ramfs_root.i_insert("proc", proc_inode.clone())?;
    let dev_dt = ramfs_root.i_insert("dev", dev_inode.clone())?;

    proc_dt.to_mount_point(procfs_root.clone(), MountFlags::empty())?;
    dev_dt.to_mount_point(devfs_root.clone(), MountFlags::empty())?;

    let path = VfsPath::new(ramfs_root.clone());
    let test1_path = path.join("d1/test1.txt")?;

    let dt1 = test1_path.create_file("rwxr-xr-x".into())?;
    let test2_path = path.join("d1/test2.txt")?;
    let dt2 = test2_path.create_file("rwxr-xr-x".into())?;

    dt1.inode()?.write_at(0, b"hello world")?;
    dt2.inode()?.write_at(0, b"test2")?;

    let proc_path = path.join("proc")?;
    let proc_dt = proc_path.open()?;

    let proc_inode = proc_dt.inode()?.downcast_arc::<ProcFsDirInodeImpl>()?;
    let pid1 = proc_inode.add_dir_manually("1", "r-xr-xr-x".into())?;
    let pid1_dt = proc_dt.i_insert("1", pid1.clone())?;
    let pid1 = pid1.downcast_arc::<ProcFsDirInodeImpl>()?;
    let pid1pid =
        pid1.add_file_manually("pid", Arc::new(ProcessInfo::new(1)), "r--r--r--".into())?;
    pid1_dt.i_insert("pid", pid1pid)?;

    info!("ramfs tree:");
    print_fs_tree(ramfs_root.clone(), "".to_string())?;
    Ok(())
}

fn register_all_fs() {
    let procfs = Arc::new(DynFs::<_, Mutex<()>>::new(
        DynFsKernelProviderImpl,
        "procfs",
    ));
    let sysfs = Arc::new(DynFs::<_, Mutex<()>>::new(DynFsKernelProviderImpl, "sysfs"));
    let ramfs = Arc::new(RamFs::<_, Mutex<()>>::new(RamFsProviderImpl));

    let devfs = Arc::new(DevFs::<_, Mutex<()>>::new(DevFsKernelProviderImpl));

    FS.lock().insert("procfs".to_string(), procfs);
    FS.lock().insert("sysfs".to_string(), sysfs);
    FS.lock().insert("ramfs".to_string(), ramfs);
    FS.lock().insert("devfs".to_string(), devfs);
    info!("register all fs");
}