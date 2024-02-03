use alloc::format;
use alloc::string::{String, ToString};
use lwext4_rs::{Error, FileType};
use vfscore::error::VfsError;
use vfscore::utils::VfsNodeType;
use vfscore::VfsResult;

pub fn from_vfs(err: VfsError) -> Error {
    match err {
        VfsError::PermissionDenied => Error::PermissionDenied,
        VfsError::NoEntry => Error::NoEntry,
        VfsError::EINTR => Error::InvalidError,
        VfsError::IoError => Error::Io,
        VfsError::EAGAIN => Error::InvalidError,
        VfsError::NoMem => Error::OutOfMemory,
        VfsError::Access => Error::PermissionDenied,
        VfsError::EBUSY => Error::InvalidError,
        VfsError::EExist => Error::FileExists,
        VfsError::NotDir => Error::NotDirectory,
        VfsError::Invalid => Error::InvalidArgument,
        VfsError::NoDev => Error::NoDevice,
        VfsError::IsDir => Error::IsDirectory,
        VfsError::NoTTY => Error::InvalidError,
        VfsError::NoSpace => Error::NoSpace,
        VfsError::ESPIPE => Error::InvalidError,
        VfsError::EPIPE => Error::InvalidError,
        VfsError::NameTooLong => Error::TooBig,
        VfsError::NoSys => Error::NotSupported,
        VfsError::NotEmpty => Error::InvalidError,
    }
}

pub fn into_vfs(err: Error) -> VfsError {
    match err {
        Error::PermissionDenied => VfsError::PermissionDenied,
        Error::NoEntry => VfsError::NoEntry,
        Error::InvalidError => VfsError::Invalid,
        Error::Io => VfsError::IoError,
        Error::OutOfMemory => VfsError::NoMem,
        Error::FileExists => VfsError::EExist,
        Error::NotDirectory => VfsError::NotDir,
        Error::InvalidArgument => VfsError::Invalid,
        Error::NoDevice => VfsError::NoDev,
        Error::IsDirectory => VfsError::IsDir,
        Error::NoSpace => VfsError::NoSpace,
        Error::TooBig => VfsError::NameTooLong,
        Error::NotSupported => VfsError::NoSys,
        _ => VfsError::Invalid,
    }
}

pub fn into_vfs_node_type(ty: FileType) -> VfsNodeType {
    if ty.is_dir() {
        VfsNodeType::Dir
    } else if ty.is_file() {
        VfsNodeType::File
    } else if ty.is_symlink() {
        VfsNodeType::SymLink
    } else if ty.is_char_device() {
        VfsNodeType::CharDevice
    } else if ty.is_block_device() {
        VfsNodeType::BlockDevice
    } else if ty.is_fifo() {
        VfsNodeType::Fifo
    } else if ty.is_socket() {
        VfsNodeType::Socket
    } else {
        VfsNodeType::Unknown
    }
}

pub fn into_file_type(ty: VfsNodeType) -> VfsResult<FileType> {
    match ty {
        VfsNodeType::Dir => Ok(FileType::from_char('d')),
        VfsNodeType::File => Ok(FileType::from_char('-')),
        VfsNodeType::SymLink => Ok(FileType::from_char('l')),
        VfsNodeType::CharDevice => Ok(FileType::from_char('c')),
        VfsNodeType::BlockDevice => Ok(FileType::from_char('b')),
        VfsNodeType::Fifo => Ok(FileType::from_char('p')),
        VfsNodeType::Socket => Ok(FileType::from_char('s')),
        VfsNodeType::Unknown => Err(VfsError::Invalid),
    }
}

pub(crate) trait Parent {
    fn parent(&self) -> Option<String>;
}

impl<T: AsRef<str>> Parent for T {
    fn parent(&self) -> Option<String> {
        let path = self.as_ref();
        let path = path.rsplit_once('/').map(|(a, _)| a.to_string());
        path
    }
}

pub(crate) trait ToDir {
    fn to_dir(&self) -> String;
}

impl<T: AsRef<str>> ToDir for T {
    fn to_dir(&self) -> String {
        let path = self.as_ref();
        if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{}/", path)
        }
    }
}

#[test]
fn test_parent() {
    assert_eq!("/a/b".parent(), Some("/a".to_string()));
    assert_eq!("/a/b/..".parent(), Some("/a/b".to_string()));
}
