trust::trusted_model! {
    fn external_hash(xs: &[u8]) -> [u8; 32];
}

#[cfg(test)]
mod tests {
    #[test]
    fn unused_stub_builds() {}
}
