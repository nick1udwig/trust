#[trust::module]
mod verified {
    trust::total! {
        given executable {
            i < xs.len();
        }

        pub fn get(xs: &[i32], i: usize) -> i32 {
            xs[i]
        }
    }

    trust::total! {
        pub fn get_or_zero(xs: &[i32], i: usize) -> i32 {
            if i < xs.len() {
                xs[i]
            } else {
                0
            }
        }
    }

    trust::total! {
        pub fn guarded_get(xs: &[i32], i: usize) -> i32 {
            if i < xs.len() {
                get(xs, i)
            } else {
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn guarded_slice_and_call_use_path_conditions() {
        assert_eq!(super::verified::get_or_zero(&[4, 8, 15], 1), 8);
        assert_eq!(super::verified::get_or_zero(&[4, 8, 15], 9), 0);
        assert_eq!(super::verified::guarded_get(&[4, 8, 15], 1), 8);
        assert_eq!(super::verified::guarded_get(&[4, 8, 15], 9), 0);
    }
}
