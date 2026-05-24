fn helper(x: i32) -> i32 {
    x
}

#[trust::module]
mod verified {
    trust::total! {
        pub fn call_helper(x: i32) -> i32 {
            super::helper(x)
        }
    }
}
