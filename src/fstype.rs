use crate::superblock::SuperType;

pub trait FsType{

    /// Determines how superblocks for this file system type are keyed.
    const SUPER_TYPE: SuperType;

    /// The name of the file system type.
    const NAME: &'static str;
}