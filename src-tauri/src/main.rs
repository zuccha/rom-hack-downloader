// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

fn flatten_directory(dir_path: &Path) -> Result<(), String> {
  let mut dir = match dir_path.read_dir() {
    Err(_) => return Err("Failed to read directory".into()),
    Ok(o) => o
  };

  // If directory is empty, do nothing.
  let entry = match dir.next() {
    None => return Ok(()),
    Some(Err(_)) => return Err("Failed to read directory".into()),
    Some(Ok(o)) => o
  };

  // If directory contains more than one entry, do nothing.
  if dir.count() > 1 { return Ok(()) }

  // If the only entry is not a directory, do nothing.
  match entry.metadata() {
    Err(_) => return Err("Failed to read entry metadata".into()),
    Ok(o) => if !o.is_dir() { return Ok(()) }
  };

  // If the only entry is a directory, move all it's entries up by one level.
  let mut temp_dir_path = PathBuf::from(dir_path);
  temp_dir_path.set_file_name(dir_path.file_name().unwrap().to_str().unwrap().to_owned() + "_tmp");

  match std::fs::rename(entry.path(), &temp_dir_path) {
    Err(_) => return Err("Failed to move single sub directory to temp directory".into()),
    _s => ()
  };

  match std::fs::remove_dir(dir_path) {
    Err(_) => return Err("Failed to removes directory".into()),
    _s => ()
  };

  match std::fs::rename(&temp_dir_path, dir_path) {
    Err(_) => return Err("Failed to move temp directory to directory".into()),
    _s => ()
  };

  Ok(())
}

#[tauri::command]
fn path_exists(path: &str) -> bool {
  Path::new(path).exists()
}

#[tauri::command]
fn validate_name(name: &str) -> Result<(), String> {
  if name.is_empty() { return Err("No name has been specified".into()) }
  Ok(())
}

#[tauri::command]
fn validate_directory_path(path: &str) -> Result<(), String> {
  if path.is_empty() { return Err("No directory has been specified".into()) }
  if !Path::new(path).exists() { return Err("Directory doesn't exist".into()) }
  Ok(())
}

#[tauri::command]
fn validate_vanilla_rom_path(path: &str) -> Result<(), String> {
  if path.is_empty() { return Err("No vanilla ROM has been specified".into()) }
  if !Path::new(path).exists() { return Err("Vanilla ROM doesn't exist".into()) }
  Ok(())
}

#[tauri::command]
fn validate_url(url: &str) -> Result<(), String> {
  if url.is_empty() { return Err("No URL has been specified".into()) }
  Ok(())
}

#[tauri::command]
async fn download_hack(
  app_handle: tauri::AppHandle,
  name: &str,
  directory_path: &str,
  url: &str,
  vanilla_rom_path: &str) -> Result<(), String> {
  // Validate Directory
  match validate_directory_path(directory_path) {
    Err(e) => return Err(e),
    _ => (),
  }

  // Validate Vanilla ROM
  match validate_vanilla_rom_path(vanilla_rom_path) {
    Err(e) => return Err(e),
    _ => (),
  }

  // Validate URL
  match validate_url(url) {
    Err(e) => return Err(e),
    _ => (),
  }

  // Build paths
  let hack_directory_path = PathBuf::from(directory_path).join(name);

  // Download
  let response = match reqwest::get(url).await {
    Err(_) => return Err("Failed to download zip".into()),
    Ok(r) => r,
  };

  let content = match response.bytes().await {
    Err(_) => return Err("Failed to download zip".into()),
    Ok(r) => std::io::Cursor::new(r)
  };

  // Unzip
  match zip_extract::extract(content, hack_directory_path.as_path(), false) {
    Err(_) => return Err("Failed to extract zip".into()),
    _ => ()
  };

  // Flatten directory if necessary
  match flatten_directory(&hack_directory_path) {
    Err(e) => return Err("Failed to flatten hack directory: ".to_string() + &e),
    _ => ()
  };

  // Retrieve Flips
  let flips_command = match std::env::consts::OS {
    "macos" => "flips-macos",
    "windows" => "flips-windows.exe",
    _ => ""
  };

  let flips_path = match app_handle
    .path_resolver()
    .resolve_resource(Path::new("resources").join(flips_command)) {
    None => return Err("Failed to locate Flips".into()),
    Some(p) => p
  };

  if !flips_path.exists() {
    return Err("Failed to retrieve Flips".into())
  }

  // Patch
  hack_directory_path.read_dir().unwrap()
    .filter_map(|res| res.ok())
    .map(|dir_entry| dir_entry.path())
    .filter(|path| path.extension().map_or(false, |ext| ext == "bps"))
    .for_each(|bps_path| {
      let mut sfc_path = PathBuf::from(&bps_path);
      sfc_path.set_extension("sfc");
      std::process::Command::new(&flips_path)
        .arg("--apply")
        .arg(&bps_path)
        .arg(&vanilla_rom_path)
        .arg(&sfc_path)
        .spawn()
        .unwrap();
    });

  // Open hack directory
  let open_command = match std::env::consts::OS {
    "macos" => "open",
    "windows" => "explorer",
    _ => ""
  };
  if !open_command.is_empty() {
    std::process::Command::new(open_command).arg(hack_directory_path).spawn().unwrap();
  }
  Ok(())
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      download_hack,
      path_exists,
      validate_directory_path,
      validate_name,
      validate_url,
      validate_vanilla_rom_path,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
