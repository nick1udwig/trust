#[trust::module]
mod verified {
    trust::total! {
        pub fn add_if_safe(x: i32) -> i32 {
            if x < i32::MAX {
                x + 1
            } else {
                x
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn add_if_safe_uses_guarded_branch() {
        assert_eq!(super::verified::add_if_safe(41), 42);
        assert_eq!(super::verified::add_if_safe(i32::MAX), i32::MAX);
    }
}
