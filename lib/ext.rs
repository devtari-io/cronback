pub trait OptionExt {
    type Item;
    fn unwrap_ref(&self) -> &Self::Item;
}
impl<T> OptionExt for Option<T> {
    type Item = T;

    fn unwrap_ref(&self) -> &T {
        self.as_ref().unwrap()
    }
}

// re-export Option extension for Prost lazy defaults
pub use dto::traits::ProstOptionExt;
