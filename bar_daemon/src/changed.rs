pub trait Changed {
    type ChangedType;

    fn changed(&self, other: &Self) -> Self::ChangedType;
}

pub trait ChangedConstructor {
    fn all_true() -> Self;
    fn all_false() -> Self;
}
