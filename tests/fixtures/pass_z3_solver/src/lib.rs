#[trust::module]
mod verified {
    trust::total! {
        pub fn id_i32(x: i32) -> i32 { x }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn id_i32_works() {
        assert_eq!(super::verified::id_i32(7), 7);
    }
}
