#[trust::module]
mod verified {
    trust::total! {
        gives ghost |out| {
            out == 0;
        }

        pub fn cfg_zero(x: i32) -> i32 {
            #[cfg(any())]
            {
                x + 1
            }

            #[cfg(not(any()))]
            {
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cfg_zero_uses_enabled_branch() {
        assert_eq!(super::verified::cfg_zero(i32::MAX), 0);
    }
}
