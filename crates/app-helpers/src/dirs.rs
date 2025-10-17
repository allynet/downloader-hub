use std::{fs, path::PathBuf};

use crate::{config::HelpersConfig, id::time_thread_id};

pub fn create_temp_dir() -> anyhow::Result<PathBuf> {
    let id = time_thread_id();
    let temp_dir = HelpersConfig::global().get_cache_dir().join(id);

    fs::create_dir_all(&temp_dir)?;

    Ok(temp_dir)
}
