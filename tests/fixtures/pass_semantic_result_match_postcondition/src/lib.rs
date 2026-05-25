#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == 0;
        }

        pub fn zero_for_any_result(x: Result<i32, i32>) -> i32 {
            match x {
                Ok(_) => 0,
                Err(_) => 0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_zero_for_all_variants() {
        assert_eq!(super::verified::zero_for_any_result(Ok(5)), 0);
        assert_eq!(super::verified::zero_for_any_result(Err(2)), 0);
    }
}
