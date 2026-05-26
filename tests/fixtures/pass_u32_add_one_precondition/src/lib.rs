#[trust::module]
mod verified {
    trust::total! {
        given executable {
            x < u32::MAX;
        }

        pub fn add_one_u32(x: u32) -> u32 { x + 1 }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn add_one_u32_works() {
        assert_eq!(super::verified::add_one_u32(41), 42);
    }

    #[test]
    #[should_panic(expected = "Trust precondition failed in add_one_u32: x < u32::MAX")]
    fn add_one_u32_checks_boundary() {
        super::verified::add_one_u32(u32::MAX);
    }
}
