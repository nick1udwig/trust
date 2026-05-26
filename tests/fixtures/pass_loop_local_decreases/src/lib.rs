#[trust::module]
mod verified {
    trust::total! {
        gives executable |out| {
            out == 0;
        }

        pub fn countdown_local(n: usize) -> usize {
            let mut i = n;
            trust::loop_spec! {
                invariant(i >= 0);
                decreases(i);
            }
            while i > 0 {
                i -= 1;
            }
            i
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn counts_down() {
        assert_eq!(super::verified::countdown_local(4), 0);
    }
}
