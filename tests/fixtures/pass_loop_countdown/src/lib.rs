#[trust::module]
mod verified {
    trust::total! {
        gives executable |out| {
            out == 0;
        }

        pub fn countdown(mut n: usize) -> usize {
            trust::loop_spec! {
                invariant(n >= 0);
                decreases(n);
            }
            while n > 0 {
                n = n - 1;
            }
            n
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn countdown_reaches_zero() {
        assert_eq!(super::verified::countdown(3), 0);
    }
}
