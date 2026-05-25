#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == 0;
        }

        pub fn zero_for_any_option(x: Option<i32>) -> i32 {
            match x {
                Some(_) => 0,
                None => 0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_zero_for_all_variants() {
        assert_eq!(super::verified::zero_for_any_option(Some(5)), 0);
        assert_eq!(super::verified::zero_for_any_option(None), 0);
    }
}
