use vfs_macro::vtable;

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
#[vtable]
pub trait SuperBlockOps{
    type Data;
}