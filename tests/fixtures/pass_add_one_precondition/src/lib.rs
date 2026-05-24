#[trust::module]
mod verified {
    trust::total! {
        given executable {
            x < i32::MAX;
        }

        pub fn add_one(x: i32) -> i32 {
            x + 1
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn add_one_works_inside_precondition() {
        assert_eq!(super::verified::add_one(41), 42);
    }

    #[test]
    #[should_panic(expected = "Trust precondition failed in add_one: x < i32::MAX")]
    fn add_one_rejects_boundary_input() {
        super::verified::add_one(i32::MAX);
    }
}
