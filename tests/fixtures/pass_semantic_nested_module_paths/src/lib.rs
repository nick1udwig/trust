#[trust::module]
mod outer {
    pub mod left {
        trust::total! {
            pub fn same(x: i32) -> i32 { x }
        }
    }

    pub mod right {
        trust::total! {
            pub fn same(x: i32) -> i32 { x }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn nested_verified_function_is_callable() {
        assert_eq!(super::outer::left::same(7), 7);
        assert_eq!(super::outer::right::same(11), 11);
    }
}
