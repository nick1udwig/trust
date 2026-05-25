#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == x;
        }

        pub fn zero_or_self(x: i32) -> i32 {
            if x == 0 {
                0
            } else {
                x
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_zero_or_self() {
        assert_eq!(super::verified::zero_or_self(0), 0);
        assert_eq!(super::verified::zero_or_self(7), 7);
    }
}
