#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == x;
        }

        pub fn zero_or_self_via_join_copy(x: i32) -> i32 {
            let mut y = x;
            if x == 0 {
                y = 0;
            }
            let z = y;
            z
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_zero_or_self() {
        assert_eq!(super::verified::zero_or_self_via_join_copy(0), 0);
        assert_eq!(super::verified::zero_or_self_via_join_copy(7), 7);
    }
}
