#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == x;
        }

        pub fn nested_match_zero_or_self(x: i32, choice: Option<i32>) -> i32 {
            if x == 0 {
                match choice {
                    Some(_) => 0,
                    None => 0,
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
        assert_eq!(super::verified::nested_match_zero_or_self(0, Some(1)), 0);
        assert_eq!(super::verified::nested_match_zero_or_self(0, None), 0);
        assert_eq!(super::verified::nested_match_zero_or_self(7, Some(1)), 7);
    }
}
