#[trust::module]
mod verified {
    trust::total! {
        pub fn id<T>(x: T) -> T { x }
    }
}
