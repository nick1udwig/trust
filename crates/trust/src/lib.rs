//! User-facing Trust API.

pub use trust_macros::{loop_spec, module, proof, spec, total, trusted_model, TrustModel};
pub use trust_model::TrustModel;

#[doc(hidden)]
pub mod __rt {
    /// Runtime assertion for executable preconditions on Rust-callable Trust functions.
    pub fn assert_precondition(condition: bool, function: &str, text: &str) {
        assert!(
            condition,
            "Trust precondition failed in {}: {}",
            function, text
        );
    }

    /// Runtime assertion for executable postconditions.
    pub fn assert_postcondition(condition: bool, function: &str, text: &str) {
        assert!(
            condition,
            "Trust postcondition failed in {}: {}; this indicates a Trust verification bug",
            function, text
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[should_panic(expected = "Trust precondition failed in get: i < xs.len()")]
    fn precondition_message_format() {
        crate::__rt::assert_precondition(false, "get", "i < xs.len()");
    }

    #[test]
    #[should_panic(
        expected = "Trust postcondition failed in f: out == x; this indicates a Trust verification bug"
    )]
    fn postcondition_message_format() {
        crate::__rt::assert_postcondition(false, "f", "out == x");
    }
}
