#[trust::module]
mod verified {
    trust::total! {
        pub fn unwrap_or_zero(x: Result<i32, i32>) -> i32 {
            match x {
                Ok(v) => v,
                Err(_) => 0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn result_match_works() {
        assert_eq!(super::verified::unwrap_or_zero(Ok(7)), 7);
        assert_eq!(super::verified::unwrap_or_zero(Err(1)), 0);
    }
}
