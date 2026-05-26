#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == x.checked_add(y);
        }

        pub fn checked_sum(x: i32, y: i32) -> Option<i32> {
            x.checked_add(y)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn checked_sum_returns_option() {
        assert_eq!(super::verified::checked_sum(2, 3), Some(5));
        assert_eq!(super::verified::checked_sum(i32::MAX, 1), None);
    }
}
