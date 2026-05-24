#[trust::module]
mod verified {
    trust::total! {
        given executable {
            x <= i32::MAX / 2;
            x >= i32::MIN / 2;
        }

        pub fn double(x: i32) -> i32 {
            x * 2
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn double_works_inside_precondition() {
        assert_eq!(super::verified::double(21), 42);
        assert_eq!(super::verified::double(-21), -42);
    }

    #[test]
    #[should_panic(expected = "Trust precondition failed in double: x <= i32::MAX / 2")]
    fn double_rejects_upper_boundary_input() {
        super::verified::double(i32::MAX / 2 + 1);
    }
}
