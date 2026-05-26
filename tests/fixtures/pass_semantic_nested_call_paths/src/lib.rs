#[trust::module]
mod outer {
    pub mod left {
        trust::total! {
            given executable { x > 0; }
            pub fn inc(x: i32) -> i32 { x }
        }

        trust::total! {
            given executable { x > 0; }
            pub fn caller(x: i32) -> i32 { inc(x) }
        }
    }

    pub mod right {
        trust::total! {
            given executable { x < 0; }
            pub fn inc(x: i32) -> i32 { x }
        }

        trust::total! {
            given executable { x < 0; }
            pub fn caller(x: i32) -> i32 { inc(x) }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn nested_callers_use_their_local_callee_contracts() {
        assert_eq!(super::outer::left::caller(7), 7);
        assert_eq!(super::outer::right::caller(-7), -7);
    }
}
