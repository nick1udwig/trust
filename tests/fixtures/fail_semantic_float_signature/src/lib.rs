#[trust::module]
mod verified {
    trust::total! {
        pub fn id_f32(x: f32) -> f32 {
            x
        }
    }
}
