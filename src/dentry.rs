pub trait DentryOps {
    type Data;
    fn d_revalidate(_: Self::Data) -> bool;
}
