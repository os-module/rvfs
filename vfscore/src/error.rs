use core::error::Error;
use core::fmt::{Debug, Display, Formatter};

#[allow(unused)]
#[derive(Debug)]
pub enum VfsError {
    /// EACCES 权限拒绝
    PermissionDenied = 13,
    /// ENOENT 无此文件或目录
    NoEntry = 2,
    /// EIO 输入输出错误
    IoError = 5,
    /// EEXIST 文件已存在
    FileExist = 17,
    /// ENOTDIR 不是目录
    NotDir = 20,
    /// ENOTEMPTY  目录非空
    NotEmpty = 39,
    /// ENOMEM 内存不足
    NoMem = 12,
    /// ENOSPC 空间不足
    NoSpace = 28,
    /// EINVAL 无效参数
    Invalid = 22,
    /// ENAMETOOLONG 名称太长
    NameTooLong = 36,
    /// ENOSYS 不支持的系统调用
    NoSys = 38,
    /// ENODEV 设备不存在
    NoDev = 19,
    /// ENOTTY 不是终端
    NoTTY = 25,
}

impl Display for VfsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            VfsError::PermissionDenied => {
                write!(f, "Permission denied")
            }
            VfsError::NoEntry => {
                write!(f, "No such file or directory")
            }
            VfsError::IoError => {
                write!(f, "Input/output error")
            }
            VfsError::FileExist => {
                write!(f, "File exists")
            }
            VfsError::NotDir => {
                write!(f, "Not a directory")
            }
            VfsError::NotEmpty => {
                write!(f, "Directory not empty")
            }
            VfsError::NoMem => {
                write!(f, "Out of memory")
            }
            VfsError::NoSpace => {
                write!(f, "No space left on device")
            }
            VfsError::Invalid => {
                write!(f, "Invalid argument")
            }
            VfsError::NameTooLong => {
                write!(f, "File name too long")
            }
            VfsError::NoSys => {
                write!(f, "Function not implemented")
            }
            VfsError::NoDev => {
                write!(f, "No such device")
            }
            VfsError::NoTTY => {
                write!(f, "Inappropriate ioctl for device")
            }
        }
    }
}

impl Error for VfsError {}


impl From<VfsError> for i32{
    fn from(value: VfsError) -> Self {
        value as i32
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_vfs_error() {
        assert_eq!(crate::error::VfsError::NoEntry as i32, 2);
    }
}
