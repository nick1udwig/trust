#[trust::module]
mod verified {
    trust::total! {
        pub fn recurse(x: i32) -> i32 {
            recurse(x)
        }
    }
}
