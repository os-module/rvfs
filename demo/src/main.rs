#![feature(seek_stream_len)]

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
use lwext4_vfs::{ExtFs, ExtFsType};
use vfscore::error::VfsError;
use vfscore::fstype::VfsFsType;
use vfscore::path::{print_fs_tree, VfsPath};
use vfscore::utils::{VfsInodeMode, VfsNodeType};
use crate::extfs::{ExtFsProviderImpl, init_extfs};

use crate::procfs::{init_procfs, DynFsKernelProviderImpl, ProcFsDirInodeImpl, ProcessInfo};
use crate::ramfs::{init_ramfs, RamFsProviderImpl};

mod devfs;
mod procfs;
mod ramfs;
mod extfs;

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
    ramfs_root.inode()?
        .create("ext", VfsNodeType::Dir, "rwxr-xr-x".into(), None)?;
    let path = VfsPath::new(ramfs_root.clone());
    path.join("proc")?.mount(procfs_root, 0)?;
    path.join("dev")?.mount(devfs_root, 0)?;
    path.join("ext")?.mount(extfs_root, 0)?;
    let test1_path = path.join("d1/test1.txt")?;

    let dt1 = test1_path.open(Some(
        VfsInodeMode::from_bits_truncate(0o777) | VfsInodeMode::FILE,
    ))?;
    let test2_path = path.join("d1/test2.txt")?;
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

    info!("ramfs tree:");
    print_fs_tree(&mut OutPut, ramfs_root.clone(), "".to_string(), true)?;
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
    let extfs = Arc::new(ExtFs::<_,Mutex<()>>::new(ExtFsType::Ext3,ExtFsProviderImpl));

    FS.lock().insert("procfs".to_string(), procfs);
    FS.lock().insert("sysfs".to_string(), sysfs);
    FS.lock().insert("ramfs".to_string(), ramfs);
    FS.lock().insert("devfs".to_string(), devfs);
    FS.lock().insert("extfs".to_string(), extfs);
    info!("register all fs");
}
