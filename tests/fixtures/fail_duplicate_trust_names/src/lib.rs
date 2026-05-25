#[trust::module]
mod first {
    trust::total! {
        pub fn same(x: i32) -> i32 { x }
    }
}

#[trust::module]
mod second {
    trust::total! {
        pub fn same(x: i32) -> i32 { x }
    }
}

