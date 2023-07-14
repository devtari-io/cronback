#[derive(Clone, Debug, PartialEq)]
pub enum View {
    Full,
    Compact,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelProjection<T> {
    inner: T,
    view: View,
}
