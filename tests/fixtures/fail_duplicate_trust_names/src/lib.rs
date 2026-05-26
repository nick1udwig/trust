trust::proof! {
    fn same() {}
}

trust::proof! {
    fn same() given ghost { true; } { assert(true); }
}
