#[trust::module]
mod verified {
    trust::total! {
        gives executable |out| {
            out == 2;
        }

        pub fn choose(x: i32) -> i32 {
            let mut y = 0;
            if x > 0 {
                y = 1;
            } else {
                y = 2;
            }
            y
        }
    }
}
