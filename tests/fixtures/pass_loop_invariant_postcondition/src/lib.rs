#[trust::module]
mod verified {
    trust::total! {
        gives executable |out| {
            out == n;
        }

        pub fn count_to(n: usize) -> usize {
            let mut i = 0;
            trust::loop_spec! {
                invariant(i <= n);
                decreases(n - i);
            }
            while i < n {
                i += 1;
            }
            i
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn counts_to_n() {
        assert_eq!(super::verified::count_to(4), 4);
    }
}
