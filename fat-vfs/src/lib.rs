#![cfg_attr(not(test), no_std)]
#![feature(trait_alias)]
mod device;
mod fs;
mod inode;

extern crate alloc;

use crate::device::FatDevice;
use alloc::string::String;
use alloc::sync::Arc;
use core::fmt::{Debug, Formatter};
use fatfs::{DefaultTimeProvider, Dir, File, LossyOemCpConverter};
use lock_api::Mutex;
use vfscore::utils::VfsTimeSpec;

pub use fs::FatFs;

pub trait VfsRawMutex = lock_api::RawMutex + Send + Sync;

pub trait FatFsProvider: Send + Sync + Clone {
    fn current_time(&self) -> VfsTimeSpec;
}

type FatDir = Dir<FatDevice, DefaultTimeProvider, LossyOemCpConverter>;
type FatFile = File<FatDevice, DefaultTimeProvider, LossyOemCpConverter>;

/// Description:
///
/// Because the fat-vfs dont support inode,so we need save some information in inode.
/// According to the information in inode,we can get the find the file in fat-vfs.
/// The information include the file name because original filesystem's inode include the inode number
/// that can identify the file uniquely but fat-vfs dont have inode number.
pub struct FatInode<R: VfsRawMutex> {
    // parent
    pub parent: Arc<Mutex<R, FatDir>>,
    // self: if the file is a directory,then the self is the directory's DIR struct.
    pub current: FatInodeType<R>,
}

pub enum FatInodeType<R: VfsRawMutex> {
    Dir(Arc<Mutex<R, FatDir>>),
    File((String, Option<Arc<Mutex<R, FatFile>>>)),
}

impl<R: VfsRawMutex> FatInode<R> {
    pub fn new(parent: Arc<Mutex<R, FatDir>>, current: FatInodeType<R>) -> Self {
        Self { parent, current }
    }
}

impl<R: VfsRawMutex> Debug for FatInode<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let current = &self.current;
        match current {
            FatInodeType::Dir(_) => f.write_str("FatInode::Dir"),
            FatInodeType::File(_) => f.write_str("FatInode::File"),
        }
    }
}
