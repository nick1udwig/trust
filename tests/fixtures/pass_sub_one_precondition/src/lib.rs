#[trust::module]
mod verified {
    trust::total! {
        given executable {
            x > i32::MIN;
        }

        pub fn sub_one(x: i32) -> i32 {
            x - 1
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sub_one_works_inside_precondition() {
        assert_eq!(super::verified::sub_one(42), 41);
    }

    #[test]
    #[should_panic(expected = "Trust precondition failed in sub_one: x > i32::MIN")]
    fn sub_one_rejects_boundary_input() {
        super::verified::sub_one(i32::MIN);
    }
}
