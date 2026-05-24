trust::proof! {
    fn le_refl(a: i32, b: i32)
    gives ghost {
        a <= a;
    }
    {
        assert(a <= b);
    }
}
