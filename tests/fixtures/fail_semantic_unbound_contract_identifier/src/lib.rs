#[trust::module]
mod verified {
    trust::total! {
        given ghost {
            y == x;
        }

        gives ghost |out| {
            out == y;
        }

        fn id(x: i32) -> i32 {
            x
        }
    }
}
