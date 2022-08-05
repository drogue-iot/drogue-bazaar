mod tracing;

pub fn phase1() {
    #[cfg(feature = "dotenv")]
    {
        let result = dotenv::dotenv();
        log::info!("dotenv: {result:?}");
    }
}

pub fn phase2(name: &str) {
    tracing::init_tracing(name);
}
