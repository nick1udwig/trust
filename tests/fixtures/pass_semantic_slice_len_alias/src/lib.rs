#[trust::module]
mod verified {
    trust::total! {
        pub fn get_or_zero(xs: &[i32], i: usize) -> i32 {
            let ys = xs;
            if i < ys.len() {
                xs[i]
            } else {
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn slice_alias_len_controls_index() {
        assert_eq!(super::verified::get_or_zero(&[4, 8, 15], 1), 8);
        assert_eq!(super::verified::get_or_zero(&[4, 8, 15], 9), 0);
    }
}
