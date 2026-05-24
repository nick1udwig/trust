#[trust::module]
mod verified {
    trust::total! {
        pub fn abs_nonmin(x: i32) -> i32 {
            if x < 0 { -x } else { x }
        }
    }
}
