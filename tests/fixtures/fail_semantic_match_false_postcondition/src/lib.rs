#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == 0;
        }

        pub fn unwrap_or_zero(x: Option<i32>) -> i32 {
            match x {
                Some(v) => v,
                None => 0,
            }
        }
    }
}
