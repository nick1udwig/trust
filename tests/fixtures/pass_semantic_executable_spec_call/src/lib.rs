#[trust::module]
mod verified {
    trust::spec! {
        executable fn nonempty(xs: &[i32]) -> bool {
            xs.len() > 0
        }
    }

    trust::total! {
        given executable {
            xs.len() > 0;
        }

        pub fn has_items(xs: &[i32]) -> bool {
            nonempty(xs)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn total_can_call_executable_spec() {
        assert!(super::verified::has_items(&[1]));
    }

    #[test]
    #[should_panic(expected = "Trust precondition failed in has_items: xs.len() > 0")]
    fn total_precondition_still_guards_empty_slice() {
        super::verified::has_items(&[]);
    }
}
