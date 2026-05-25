#[trust::module]
mod verified {
    trust::total! {
        given executable {
            x < i32::MAX;
        }

        pub fn inc(x: i32) -> i32 {
            x + 1
        }
    }

    trust::total! {
        given executable {
            x < i32::MAX;
        }

        pub fn caller(x: i32) -> i32 {
            inc({ x })
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn caller_uses_verified_inc() {
        assert_eq!(super::verified::caller(41), 42);
    }
}
