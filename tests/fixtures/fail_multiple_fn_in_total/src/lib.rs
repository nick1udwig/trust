#[trust::module]
mod verified {
    trust::total! {
        pub fn first(x: i32) -> i32 { x }
        pub fn second(x: i32) -> i32 { x }
    }
}
