#[trust::module]
mod left {
    use trust::TrustModel;

    #[derive(TrustModel)]
    pub struct Account {
        pub balance: i64,
    }

    trust::total! {
        pub fn same(x: i32) -> i32 { x }
    }

    trust::proof! {
        fn lemma() {}
    }
}

#[trust::module]
mod right {
    use trust::TrustModel;

    #[derive(TrustModel)]
    pub struct Account {
        pub balance: i64,
    }

    trust::total! {
        pub fn same(x: i32) -> i32 { x }
    }

    trust::proof! {
        fn lemma() {}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn both_modules_can_use_the_same_leaf_name() {
        assert_eq!(super::left::same(7), 7);
        assert_eq!(super::right::same(11), 11);
    }
}
