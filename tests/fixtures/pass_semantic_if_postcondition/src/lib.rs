#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == 0;
        }

        pub fn zero_for_any_i32(x: i32) -> i32 {
            if x > 0 {
                0
            } else {
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_zero_on_both_paths() {
        assert_eq!(super::verified::zero_for_any_i32(5), 0);
        assert_eq!(super::verified::zero_for_any_i32(-5), 0);
    }
}
