pub fn add_one(x: i32) -> i32 {
    x + 1
}

#[cfg(test)]
mod tests {
    #[test]
    fn ordinary_rust_works() {
        assert_eq!(super::add_one(1), 2);
    }
}
