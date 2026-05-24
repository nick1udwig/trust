#[trust::module]
mod verified {
    trust::total! {
        pub fn bad_expect(x: Result<i32, i32>) -> i32 {
            x.expect("ok")
        }
    }
}
