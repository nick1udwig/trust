#[trust::module]
mod verified {
    trust::total! {
        given executable {
            x < i32::MAX;
        }

        gives ghost |out| {
            int(out) == int(x) + 1;
        }

        pub fn add_one(x: i32) -> i32 {
            let y = x + 1;
            y
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn add_one_works_inside_precondition() {
        assert_eq!(super::verified::add_one(41), 42);
    }
}
