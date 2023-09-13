// use vfs_macro::vtable;

/// Type of superblock keying.
///
/// It determines how C's `fs_context_operations::get_tree` is implemented.
pub enum SuperType {
    /// Only one such superblock may exist.
    Single,

    /// As [`Super::Single`], but reconfigure if it exists.
    SingleReconf,

    /// Superblocks with different data pointers may exist.
    Keyed,

    /// Multiple independent superblocks may exist.
    Independent,

    /// Uses a block device.
    BlockDev,
}
pub trait SuperBlockOps: Send + Sync {
    type Data;
    type Context;
    /// Determines how superblocks for this file system type are keyed.
    const SUPER_TYPE: SuperType;
    fn fill_super(&self) -> Self::Context;
}
