#[trust::module]
mod verified {
    trust::total! {
        pub fn is_zero(x: i32) -> bool { x == 0 }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn is_zero_works() {
        assert!(super::verified::is_zero(0));
        assert!(!super::verified::is_zero(7));
    }
}
