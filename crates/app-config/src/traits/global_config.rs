use std::sync::OnceLock;

pub trait GlobalConfig: Sized + 'static {
    fn global_instance() -> &'static OnceLock<Self>;

    fn init_global(conf: Self) -> Result<(), String> {
        Self::global_instance()
            .set(conf)
            .map_err(|_| "Config already initialized".to_string())
    }

    #[must_use]
    #[inline]
    fn get_global() -> &'static Self {
        Self::global_instance()
            .get()
            .expect("Config not initialized")
    }
}
