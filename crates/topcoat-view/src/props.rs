/// A properties struct whose builder checks required fields at compile time.
///
/// Implemented by [`#[derive(Props)]`][derive]. Bring the trait into scope to
/// call `builder()` on the struct.
///
/// [derive]: derive.Props.html
pub trait Props {
    /// The builder type in its initial state, with no properties set.
    type Builder;

    /// Returns a builder with no properties set.
    fn builder() -> Self::Builder;
}

/// Marks a required property as set in a generated builder.
///
/// Calling a required property's setter changes its type parameter to
/// [`Set`]. See [`#[derive(Props)]`][derive].
///
/// [derive]: derive.Props.html
pub struct Set;

/// Indicates that a generated builder's property has been set.
///
/// A builder's `build()` method requires this trait for every required
/// property. Unset properties produce a compile error naming the missing field.
#[diagnostic::on_unimplemented(
    message = "missing required property `{Self}`",
    label = "the property `{Self}` was not set",
    note = "every property without `#[default]` must be set before calling `build()`"
)]
pub trait IsSet {}

impl IsSet for Set {}
