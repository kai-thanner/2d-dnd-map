use crate::config::Config;
use crate::edit::GameState;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const SAVE_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,
    pub config: Config,
    pub state: GameState,
}

pub fn save(path: &Path, cfg: &Config, state: &GameState) -> std::io::Result<()> {
    let sf = SaveFile {
        version: SAVE_VERSION,
        config: cfg.clone(),
        state: state.clone(),
    };
    let file = std::fs::File::create(path)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &sf)?;
    Ok(())
}

pub fn load(path: &Path) -> std::io::Result<(Config, GameState)> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let sf: SaveFile = serde_json::from_reader(reader)?;
    if sf.version != SAVE_VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Save-Version {} wird nicht unterstützt (erwartet: {})",
                sf.version, SAVE_VERSION
            ),
        ));
    }
    Ok((sf.config, sf.state))
}
