use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use Result;

const BLUE_FRAME: &'static [u8] = include_bytes!("blue.jpg");

pub fn blue_frame_path() -> Result<PathBuf> {
    let tmpdir = env::temp_dir();
    let blue_path = tmpdir.join("vcr-blue-frame.jpg");
    if !blue_path.exists() {
        let mut f = File::create(&blue_path)?;
        f.write_all(BLUE_FRAME)?;
        f.flush()?;
    }
    Ok(blue_path)
}
