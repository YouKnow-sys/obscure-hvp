/// a trait that can be used to share the rebuild progress with the user
pub trait RebuildProgress {
    /// incress the progress by 1
    fn inc(&self, message: Option<String>);
    /// incress the progress by n
    fn inc_n(&self, n: usize, message: Option<String>);
}
