#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == x;
        }

        pub fn nested_zero_or_self(x: i32, flag: bool) -> i32 {
            if x == 0 {
                if flag {
                    0
                } else {
                    0
                }
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
        assert_eq!(super::verified::nested_zero_or_self(0, true), 0);
        assert_eq!(super::verified::nested_zero_or_self(0, false), 0);
        assert_eq!(super::verified::nested_zero_or_self(7, true), 7);
    }
}
