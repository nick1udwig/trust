#[trust::module]
mod verified {
    trust::total! {
        given executable {
            x == None;
        }

        gives ghost |out| {
            out == 0;
        }

        pub fn zero_for_alias_option(x: Option<i32>) -> i32 {
            let y = x;
            match y {
                Some(v) => v,
                None => 0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_zero_for_none_alias() {
        assert_eq!(super::verified::zero_for_alias_option(None), 0);
    }
}
