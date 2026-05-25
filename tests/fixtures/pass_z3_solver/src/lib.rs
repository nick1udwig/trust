#[trust::module]
mod verified {
    trust::total! {
        given executable {
            x < 2147483647;
        }

        pub fn add_one(x: i32) -> i32 { x + 1 }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn add_one_works() {
        assert_eq!(super::verified::add_one(7), 8);
    }
}
