#[trust::module]
mod verified {
    trust::total! {
        given executable {
            x < i64::MAX;
        }

        pub fn add_one_i64(x: i64) -> i64 { x + 1 }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn add_one_i64_works() {
        assert_eq!(super::verified::add_one_i64(41), 42);
    }

    #[test]
    #[should_panic(expected = "Trust precondition failed in add_one_i64: x < i64::MAX")]
    fn add_one_i64_checks_boundary() {
        super::verified::add_one_i64(i64::MAX);
    }
}
