#[trust::module]
mod verified {
    trust::total! {
        pub fn unwrap_or_zero(x: Option<i32>) -> i32 {
            match x {
                Some(v) => v,
                None => 0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unwraps_some_or_returns_zero() {
        assert_eq!(super::verified::unwrap_or_zero(Some(5)), 5);
        assert_eq!(super::verified::unwrap_or_zero(None), 0);
    }
}
