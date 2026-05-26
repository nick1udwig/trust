#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == x;
        }

        pub fn id_string(x: String) -> String {
            x
        }
    }
}
