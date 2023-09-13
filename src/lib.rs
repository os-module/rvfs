#![feature(associated_type_defaults)]
#![cfg_attr(not(test), no_std)]
#![feature(error_in_core)]
extern crate alloc;

pub mod dentry;
mod error;
pub mod file;
pub mod fstype;
pub mod inode;
pub mod mount;
#[cfg(feature = "ramfs")]
pub mod ramfs;
pub mod superblock;
