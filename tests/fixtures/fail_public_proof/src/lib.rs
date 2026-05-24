trust::proof! {
    pub fn le_refl(a: i32)
    gives ghost {
        a <= a;
    }
    {
        assert(a <= a);
    }
}
