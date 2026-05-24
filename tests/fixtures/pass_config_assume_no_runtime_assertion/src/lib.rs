#[trust::module]
mod verified {
    trust::total! {
        given executable {
            x > 0;
        }

        pub fn identity(x: i32) -> i32 { x }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn executable_precondition_is_not_asserted_at_runtime() {
        assert_eq!(super::verified::identity(0), 0);
    }
}
