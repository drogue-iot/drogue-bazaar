mod tracing;

pub use self::tracing::Tracing;

pub fn phase1(dotenv: bool) {
    if dotenv {
        let result = dotenvy::dotenv();
        log::info!("dotenv: {result:?}");
    }
}

pub fn phase2(name: &str, tracing: Tracing) {
    tracing::init_tracing(name, tracing);
}
