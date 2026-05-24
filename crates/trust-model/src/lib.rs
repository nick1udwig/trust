//! Built-in Trust model marker APIs.

/// Marker trait for types with Trust structural metadata.
///
/// The derive macro is implemented in a later MVP milestone. This trait exists
/// now so generated and user-facing paths are stable from the first slice.
pub trait TrustModel {}
