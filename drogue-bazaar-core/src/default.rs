/// Evaluate if the value is equal to its types default value.
///
/// This is intended to be use with serde's `skip_serializing_if`. But keep in mind, that this will
/// create a new (default) instant of the type for every check.
pub fn is_default<T>(value: &T) -> bool
where
    T: Default + PartialEq,
{
    value == &T::default()
}
