#[trust::module]
mod verified {
    trust::total! {
        pub fn overflow() -> i32 {
            let x: i32 = i32::MAX;
            x + 1
        }
    }
}
