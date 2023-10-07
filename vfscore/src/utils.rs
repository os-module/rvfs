use alloc::string::String;
use bitflags::bitflags;
bitflags! {
    pub struct VfsInodeMode: u32 {
        // const NULL  = 0;
        /// Type
        const TYPE_MASK = 0o170000;
        /// FIFO
        const FIFO  = 0o010000;
        /// character device
        const CHAR  = 0o020000;
        /// directory
        const DIR   = 0o040000;
        /// block device
        const BLOCK = 0o060000;
        /// ordinary regular file
        const FILE  = 0o100000;
        /// symbolic link
        const LINK  = 0o120000;
        /// socket
        const SOCKET = 0o140000;

        /// Set-user-ID on execution.
        const SET_UID = 0o4000;
        /// Set-group-ID on execution.
        const SET_GID = 0o2000;

        /// Read, write, execute/search by owner.
        const OWNER_MASK = 0o700;
        /// Read permission, owner.
        const OWNER_READ = 0o400;
        /// Write permission, owner.
        const OWNER_WRITE = 0o200;
        /// Execute/search permission, owner.
        const OWNER_EXEC = 0o100;

        /// Read, write, execute/search by group.
        const GROUP_MASK = 0o70;
        /// Read permission, group.
        const GROUP_READ = 0o40;
        /// Write permission, group.
        const GROUP_WRITE = 0o20;
        /// Execute/search permission, group.
        const GROUP_EXEC = 0o10;

        /// Read, write, execute/search by others.
        const OTHER_MASK = 0o7;
        /// Read permission, others.
        const OTHER_READ = 0o4;
        /// Write permission, others.
        const OTHER_WRITE = 0o2;
        /// Execute/search permission, others.
        const OTHER_EXEC = 0o1;
    }
}

/// Node (file/directory) type.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VfsNodeType {
    Unknown = 0,
    /// FIFO (named pipe)
    Fifo = 0o1,
    /// Character device
    CharDevice = 0o2,
    /// Directory
    Dir = 0o4,
    /// Block device
    BlockDevice = 0o6,
    /// Regular file
    File = 0o10,
    /// Symbolic link
    SymLink = 0o12,
    /// Socket
    Socket = 0o14,
}

impl VfsNodeType {
    /// Tests whether this node type represents a regular file.
    pub const fn is_file(self) -> bool {
        matches!(self, Self::File)
    }

    /// Tests whether this node type represents a directory.
    pub const fn is_dir(self) -> bool {
        matches!(self, Self::Dir)
    }

    /// Tests whether this node type represents a symbolic link.
    pub const fn is_symlink(self) -> bool {
        matches!(self, Self::SymLink)
    }

    /// Returns `true` if this node type is a block device.
    pub const fn is_block_device(self) -> bool {
        matches!(self, Self::BlockDevice)
    }

    /// Returns `true` if this node type is a char device.
    pub const fn is_char_device(self) -> bool {
        matches!(self, Self::CharDevice)
    }

    /// Returns `true` if this node type is a fifo.
    pub const fn is_fifo(self) -> bool {
        matches!(self, Self::Fifo)
    }

    /// Returns `true` if this node type is a socket.
    pub const fn is_socket(self) -> bool {
        matches!(self, Self::Socket)
    }

    /// Returns a character representation of the node type.
    ///
    /// For example, `d` for directory, `-` for regular file, etc.
    pub const fn as_char(self) -> char {
        match self {
            Self::Fifo => 'p',
            Self::CharDevice => 'c',
            Self::Dir => 'd',
            Self::BlockDevice => 'b',
            Self::File => '-',
            Self::SymLink => 'l',
            Self::Socket => 's',
            _ => '?',
        }
    }
}

bitflags::bitflags! {
    /// Node (file/directory) permission mode.
    pub struct VfsNodePerm: u16 {
        /// Owner has read permission.
        const OWNER_READ = 0o400;
        /// Owner has write permission.
        const OWNER_WRITE = 0o200;
        /// Owner has execute permission.
        const OWNER_EXEC = 0o100;

        /// Group has read permission.
        const GROUP_READ = 0o40;
        /// Group has write permission.
        const GROUP_WRITE = 0o20;
        /// Group has execute permission.
        const GROUP_EXEC = 0o10;

        /// Others have read permission.
        const OTHER_READ = 0o4;
        /// Others have write permission.
        const OTHER_WRITE = 0o2;
        /// Others have execute permission.
        const OTHER_EXEC = 0o1;
    }
}

impl From<&str> for VfsNodePerm {
    fn from(val: &str) -> Self {
        let bytes = val.as_bytes();
        assert_eq!(bytes.len(), 9);
        let mut perm = VfsNodePerm::empty();
        if bytes[0] == b'r' {
            perm |= VfsNodePerm::OWNER_READ;
        }
        if bytes[1] == b'w' {
            perm |= VfsNodePerm::OWNER_WRITE;
        }
        if bytes[2] == b'x' {
            perm |= VfsNodePerm::OWNER_EXEC;
        }
        if bytes[3] == b'r' {
            perm |= VfsNodePerm::GROUP_READ;
        }
        if bytes[4] == b'w' {
            perm |= VfsNodePerm::GROUP_WRITE;
        }
        if bytes[5] == b'x' {
            perm |= VfsNodePerm::GROUP_EXEC;
        }
        if bytes[6] == b'r' {
            perm |= VfsNodePerm::OTHER_READ;
        }
        if bytes[7] == b'w' {
            perm |= VfsNodePerm::OTHER_WRITE;
        }
        if bytes[8] == b'x' {
            perm |= VfsNodePerm::OTHER_EXEC;
        }
        perm
    }
}

#[test]
fn test_perm_from_str() {
    let perm: VfsNodePerm = "rwxrwxrwx".into();
    assert_eq!(perm, VfsNodePerm::from_bits_truncate(0o777));
    let perm: VfsNodePerm = "rwxr-xr-x".into();
    assert_eq!(perm, VfsNodePerm::from_bits_truncate(0o755));
    let perm: VfsNodePerm = "rw-rw-rw-".into();
    assert_eq!(perm, VfsNodePerm::from_bits_truncate(0o666));
    let perm: VfsNodePerm = "rw-r--r--".into();
    assert_eq!(perm, VfsNodePerm::from_bits_truncate(0o644));
    let perm: VfsNodePerm = "rw-------".into();
    assert_eq!(perm, VfsNodePerm::from_bits_truncate(0o600));
    let perm: VfsNodePerm = "r--r--r--".into();
    assert_eq!(perm, VfsNodePerm::from_bits_truncate(0o444));
    let perm: VfsNodePerm = "r--------".into();
    assert_eq!(perm, VfsNodePerm::from_bits_truncate(0o400));
}

impl From<VfsInodeMode> for VfsNodeType {
    fn from(value: VfsInodeMode) -> Self {
        match value & VfsInodeMode::TYPE_MASK {
            VfsInodeMode::FIFO => VfsNodeType::Fifo,
            VfsInodeMode::CHAR => VfsNodeType::CharDevice,
            VfsInodeMode::DIR => VfsNodeType::Dir,
            VfsInodeMode::BLOCK => VfsNodeType::BlockDevice,
            VfsInodeMode::FILE => VfsNodeType::File,
            VfsInodeMode::LINK => VfsNodeType::SymLink,
            VfsInodeMode::SOCKET => VfsNodeType::Socket,
            _ => panic!("Invalid inode type"),
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn inode_mode2node_type() {
        use super::*;
        assert_eq!(VfsNodeType::Fifo, VfsInodeMode::FIFO.into());
        assert_eq!(VfsNodeType::CharDevice, VfsInodeMode::CHAR.into());
        assert_eq!(VfsNodeType::Dir, VfsInodeMode::DIR.into());
        assert_eq!(VfsNodeType::BlockDevice, VfsInodeMode::BLOCK.into());
        assert_eq!(VfsNodeType::File, VfsInodeMode::FILE.into());
        assert_eq!(VfsNodeType::SymLink, VfsInodeMode::LINK.into());
        assert_eq!(VfsNodeType::Socket, VfsInodeMode::SOCKET.into());
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct VfsTimeSpec {
    pub sec: u64,  /* 秒 */
    pub nsec: u64, /* 纳秒, 范围在0~999999999 */
}

impl VfsTimeSpec {
    pub fn new(sec: u64, nsec: u64) -> Self {
        Self { sec, nsec }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct VfsFsStat {
    /// 是个 magic number，每个知名的 fs 都各有定义，但显然我们没有
    pub f_type: i64,
    /// 最优传输块大小
    pub f_bsize: i64,
    /// 总的块数
    pub f_blocks: u64,
    /// 还剩多少块未分配
    pub f_bfree: u64,
    /// 对用户来说，还有多少块可用
    pub f_bavail: u64,
    /// 总的 inode 数
    pub f_files: u64,
    /// 空闲的 inode 数
    pub f_ffree: u64,
    /// 文件系统编号，但实际上对于不同的OS差异很大，所以不会特地去用
    pub f_fsid: [i32; 2],
    /// 文件名长度限制，这个OS默认FAT已经使用了加长命名
    pub f_namelen: isize,
    /// 片大小
    pub f_frsize: isize,
    /// 一些选项，但其实也没用到
    pub f_flags: isize,
    /// 空余 padding
    pub f_spare: [isize; 4],
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FileStat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub __pad: u64,
    pub st_size: u64,
    pub st_blksize: u32,
    pub __pad2: u32,
    pub st_blocks: u64,
    pub st_atime: VfsTimeSpec,
    pub st_mtime: VfsTimeSpec,
    pub st_ctime: VfsTimeSpec,
    pub unused: u64,
} //128

#[derive(Debug, Clone)]
pub struct VfsDirEntry {
    /// ino is an inode number
    pub ino: u64,
    /// type is the file type
    pub ty: VfsNodeType,
    /// filename (null-terminated)
    pub name: String,
}

bitflags! {
    /// ppoll 使用，表示对应在文件上等待或者发生过的事件
    pub struct PollEvents: u16 {
        /// 可读
        const IN = 0x0001;
        /// 可写
        const OUT = 0x0004;
        /// 报错
        const ERR = 0x0008;
        /// 已终止，如 pipe 的另一端已关闭连接的情况
        const HUP = 0x0010;
        /// 无效的 fd
        const INVAL = 0x0020;
    }
}
