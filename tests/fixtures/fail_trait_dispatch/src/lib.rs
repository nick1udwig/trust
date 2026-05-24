#[trust::module]
mod verified {
    trust::total! {
        pub fn display(x: i32) -> String {
            x.to_string()
        }
    }
}
