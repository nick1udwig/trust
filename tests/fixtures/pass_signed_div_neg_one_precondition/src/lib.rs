#[trust::module]
mod verified {
    trust::total! {
        given executable {
            x > i32::MIN;
        }

        pub fn div_neg_one(x: i32) -> i32 {
            x / -1
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn div_neg_one_works_inside_precondition() {
        assert_eq!(super::verified::div_neg_one(42), -42);
    }

    #[test]
    #[should_panic(expected = "Trust precondition failed in div_neg_one: x > i32::MIN")]
    fn div_neg_one_rejects_boundary_input() {
        super::verified::div_neg_one(i32::MIN);
    }
}
