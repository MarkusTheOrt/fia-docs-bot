use std::{
    borrow::Cow, path::{Path, PathBuf}, process::Stdio, str::FromStr
};

use crate::error::MagickError;

#[cfg(target_os = "windows")]
const CONVERT_COMMAND: &str = "magick";

#[cfg(not(target_os = "windows"))]
const CONVERT_COMMAND: &str = "convert";

pub fn check_magick() -> bool {
    let cmd = match std::process::Command::new("which")
        .stdout(Stdio::null())
        .arg(CONVERT_COMMAND)
        .spawn()
    {
        Ok(cmd) => cmd,
        Err(_) => return false,
    };

    match cmd.wait_with_output() {
        Ok(output) => {
            if output.status.success() {
                return true;
            }
        },
        _ => return false,
    }
    false
}

pub fn run_magick(
    input: Cow<'_, str>,
    output: &str,
) -> crate::error::Result<Vec<PathBuf>> {
    create_doc_dir(output)?;
    let cmd = std::process::Command::new(CONVERT_COMMAND)
        .args(["-density", "400"])
        .arg(format!("{input}[0-100]"))
        .args(["-alpha", "remove"])
        .args(["-quality", "95"])
        .arg(format!("./tmp/{output}/0.jpg"))
        .stdout(Stdio::null())
        .spawn()?;

    let cmd_output = cmd.wait_with_output()?;
    if !cmd_output.status.success() {
        unsafe {
            return Err(crate::error::Error::Magick(MagickError(String::from_utf8_unchecked(cmd_output.stderr))));
        }
    }
    Ok(get_converted_files(output))
}

pub fn get_converted_files(input: &str) -> Vec<PathBuf> {
    let mut output = vec![];
    let Ok(initial) = PathBuf::from_str(&format!("./tmp/{input}/0.jpg"));
    if initial.exists() {
        output.push(initial);
    }
    for i in 0..=100 {
        let path = match PathBuf::from_str(&format!("./tmp/{input}/0-{i}.jpg"))
        {
            Err(_) => continue,
            Ok(path) => path,
        };
        // if our file doesn't exist there won't be others anyways.
        if !path.exists() {
            break;
        }
        output.push(path);
    }
    output
}

pub fn create_tmp_dir() -> Result<(), std::io::Error> {
    let path = Path::new("./tmp");
    if !path.exists() {
        std::fs::create_dir("./tmp")?;
    };
    Ok(())
}

pub fn create_doc_dir(filename: &str) -> Result<(), std::io::Error> {
    let pathname = format!("./tmp/{filename}/");
    let path = Path::new(&pathname);
    if !path.exists() {
        std::fs::create_dir(pathname)?;
    }
    Ok(())
}

pub fn clear_tmp_dir() -> Result<(), std::io::Error> {
    std::fs::remove_dir_all("./tmp/")?;
    create_tmp_dir()?;
    Ok(())
}
