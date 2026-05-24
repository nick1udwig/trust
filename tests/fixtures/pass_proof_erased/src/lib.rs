trust::proof! {
    fn le_trans(a: i32, b: i32, c: i32)
    given ghost {
        a <= b;
        b <= c;
    }
    gives ghost {
        a <= c;
    }
    {
        assert(a <= c);
    }
}

pub fn ordinary_rust_still_builds() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    #[test]
    fn proof_is_erased() {
        assert_eq!(super::ordinary_rust_still_builds(), 1);
    }
}
