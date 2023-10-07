use std::collections::HashMap;
use std::error::Error;
use std::ops::Index;
use std::sync::Arc;
use log::info;
use spin::{Lazy, Mutex};
use ::devfs::DevFs;
use ::ramfs::RamFs;
use dynfs::DynFs;
use vfscore::fstype::{MountFlags, VfsFsType};
use vfscore::utils::VfsNodeType;
use crate::devfs::{DevFsKernelProviderImpl, init_devfs};

use crate::procfs::{DynFsKernelProviderImpl, init_procfs};
use crate::ramfs::{init_ramfs, RamFsProviderImpl};
use crate::utils::print_fs_tree;

mod procfs;
mod ramfs;
mod devfs;
mod utils;


static FS:Lazy<Mutex<HashMap<String,Arc<dyn VfsFsType>>>> = Lazy::new(||Mutex::new(HashMap::new()));

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    register_all_fs();
    let ramfs_root = init_ramfs(FS.lock().index("ramfs").clone())?;
    let procfs_root = init_procfs(FS.lock().index("procfs").clone())?;
    let devfs_root  = init_devfs(FS.lock().index("devfs").clone())?;


    let proc_inode=  ramfs_root.inode()?.create("proc", VfsNodeType::Dir, "rwxr-xr-x".into(), None)?;
    let dev_inode=  ramfs_root.inode()?.create("dev", VfsNodeType::Dir, "rwxr-xr-x".into(), None)?;
    let proc_dt = ramfs_root.clone().insert("proc",proc_inode.clone())?;
    let dev_dt = ramfgsts_root.clone().insert("dev",dev_inode.clone())?;

    proc_dt.to_mount_point(procfs_root.clone(),MountFlags::empty())?;
    dev_dt.to_mount_point(devfs_root.clone(),MountFlags::empty())?;
    info!("ramfs tree:");
    print_fs_tree(ramfs_root,"-".to_string());
    Ok(())
}


fn register_all_fs(){
    let procfs = Arc::new(DynFs::<_, Mutex<()>>::new(
        DynFsKernelProviderImpl,
        "procfs",
    ));
    let sysfs = Arc::new(DynFs::<_, Mutex<()>>::new(
        DynFsKernelProviderImpl,
        "sysfs",
    ));
    let ramfs = Arc::new(RamFs::<_,Mutex<()>>::new(RamFsProviderImpl));

    let devfs = Arc::new(DevFs::<_,Mutex<()>>::new(DevFsKernelProviderImpl));

    FS.lock().insert("procfs".to_string(),procfs);
    FS.lock().insert("sysfs".to_string(),sysfs);
    FS.lock().insert("ramfs".to_string(),ramfs);
    FS.lock().insert("devfs".to_string(),devfs);
    info!("register all fs");
}