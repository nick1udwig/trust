#[trust::module]
mod verified {
    trust::total! {
        pub fn keep_i32(x: i32) -> i32 {
            let y = 1.0f32;
            let _ = y;
            x
        }
    }
}
