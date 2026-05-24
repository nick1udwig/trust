#[trust::module]
mod verified {
    trust::total! {
        given executable {
            x > i32::MIN;
        }

        pub fn abs_nonmin(x: i32) -> i32 {
            if x < 0 { -x } else { x }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn abs_nonmin_works_inside_precondition() {
        assert_eq!(super::verified::abs_nonmin(-7), 7);
        assert_eq!(super::verified::abs_nonmin(7), 7);
    }

    #[test]
    #[should_panic(expected = "Trust precondition failed in abs_nonmin: x > i32::MIN")]
    fn abs_nonmin_rejects_min_value() {
        super::verified::abs_nonmin(i32::MIN);
    }
}
