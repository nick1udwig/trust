trust::proof! {
    fn le_refl(a: i32)
    gives ghost {
        a <= a;
    }
    {
        println!("runtime");
        assert(a <= a);
    }
}
