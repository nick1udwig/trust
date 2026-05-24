#[trust::module]
mod verified {
    trust::total! {
        pub fn apply(x: i32) -> i32 {
            let inc = |n: i32| n + 1;
            inc(x)
        }
    }
}
