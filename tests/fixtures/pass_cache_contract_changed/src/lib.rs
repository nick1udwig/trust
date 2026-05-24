#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == x;
        }

        pub fn id_i32(x: i32) -> i32 {
            x
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn identity_returns_input() {
        assert_eq!(super::verified::id_i32(7), 7);
    }
}
