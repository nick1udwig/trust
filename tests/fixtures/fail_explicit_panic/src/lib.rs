#[trust::module]
mod verified {
    trust::total! {
        pub fn fail() -> i32 {
            panic!("boom")
        }
    }
}
