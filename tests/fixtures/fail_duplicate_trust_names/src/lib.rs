#[trust::module]
mod first {
    trust::proof! {
        fn same() {}
    }
}

#[trust::module]
mod second {
    trust::proof! {
        fn same() {}
    }
}
