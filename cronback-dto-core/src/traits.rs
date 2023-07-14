pub trait ProstLazyDefault {
    fn default_instance() -> &'static Self;
}

pub trait ProstOptionExt {
    type Item;
    fn get_or_default(&self) -> &Self::Item;
}

impl<T: ProstLazyDefault + 'static> ProstOptionExt for Option<T> {
    type Item = T;

    fn get_or_default(&self) -> &T {
        self.as_ref().unwrap_or_else(|| T::default_instance())
    }
}
