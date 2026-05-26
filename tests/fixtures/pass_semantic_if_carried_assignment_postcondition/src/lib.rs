#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == x;
        }

        pub fn zero_or_self_via_carried_local(x: i32) -> i32 {
            let mut y = x;
            if x == 0 {
                y = 0;
            }
            y
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_zero_or_self() {
        assert_eq!(super::verified::zero_or_self_via_carried_local(0), 0);
        assert_eq!(super::verified::zero_or_self_via_carried_local(7), 7);
    }
}
