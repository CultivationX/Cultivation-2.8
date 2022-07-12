#![cfg_attr(
all(not(debug_assertions), target_os = "windows"),
windows_subsystem = "windows"
)]

use lazy_static::lazy_static;
use std::{sync::Mutex, collections::HashMap};
use std::path::PathBuf;

use std::thread;
use sysinfo::{System, SystemExt};
use structs::{APIQuery};

mod structs;
mod system_helpers;
mod file_helpers;
mod unzip;
mod downloader;
mod lang;
mod proxy;
mod web;

lazy_static! {
  static ref WATCH_GAME_PROCESS: Mutex<String> = {
      let m = "".to_string();
      Mutex::new(m)
  };
}

fn main() {
  // Start the game process watcher.
  process_watcher();

  // Make BG folder if it doesn't exist.
  let bg_folder: PathBuf = [&system_helpers::install_location(), "bg"].iter().collect();
  std::fs::create_dir_all(&bg_folder).unwrap();

  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      enable_process_watcher,
      connect,
      disconnect,
      req_get,
      get_bg_file,
      base64_decode,
      is_game_running,
      get_theme_list,
      system_helpers::run_command,
      system_helpers::run_program,
      system_helpers::run_jar,
      system_helpers::open_in_browser,
      system_helpers::copy_file,
      system_helpers::install_location,
      system_helpers::is_elevated,
      proxy::set_proxy_addr,
      proxy::generate_ca_files,
      unzip::unzip,
      file_helpers::rename,
      file_helpers::dir_exists,
      file_helpers::dir_is_empty,
      file_helpers::dir_delete,
      downloader::download_file,
      downloader::stop_download,
      lang::get_lang,
      lang::get_languages,
      web::valid_url
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

fn process_watcher() {
  // Every 5 seconds, see if the game process is still running.
  // If it is not, then we assume the game has closed and disable the proxy
  // to prevent any requests from being sent to the game.

  // Start a thread so as to not block the main thread.
  thread::spawn(|| {
    let mut system = System::new_all();

    loop {
      // Refresh system info
      system.refresh_all();

      // Grab the game process name
      let proc = WATCH_GAME_PROCESS.lock().unwrap().to_string();

      if !&proc.is_empty() {
        let proc_with_name = system.processes_by_exact_name(&proc);
        let mut exists = false;

        for _p in proc_with_name {
          exists = true;
          break;
        }

        // If the game process closes, disable the proxy.
        if !exists {
          *WATCH_GAME_PROCESS.lock().unwrap() = "".to_string();
          disconnect();
        }
      }
      thread::sleep(std::time::Duration::from_secs(5));
    }
  });
}

#[tauri::command]
fn is_game_running() -> bool {
  // Grab the game process name
  let proc = WATCH_GAME_PROCESS.lock().unwrap().to_string();

  return !proc.is_empty();
}

#[tauri::command]
fn enable_process_watcher(process: String) {
  *WATCH_GAME_PROCESS.lock().unwrap() = process;
}

#[tauri::command]
async fn connect(port: u16, certificate_path: String) {
  // Log message to console.
  println!("Connecting to proxy...");

  // Change proxy settings.
  proxy::connect_to_proxy(port);

  // Create and start a proxy.
  proxy::create_proxy(port, certificate_path).await;
}

#[tauri::command]
fn disconnect() {
  // Log message to console.
  println!("Disconnecting from proxy...");

  // Change proxy settings.
  proxy::disconnect_from_proxy();
}

#[tauri::command]
async fn req_get(url: String) -> String {
  // Send a GET request to the specified URL.
  let response = web::query(&url.to_string()).await;

  // Send the response body back to the client.
  return response;
}

#[tauri::command]
async fn get_theme_list(data_dir: String) -> Vec<HashMap<String, String>> {
  let theme_loc = format!("{}/themes", data_dir);

  // Ensure folder exists
  if !std::path::Path::new(&theme_loc).exists() {
    std::fs::create_dir_all(&theme_loc).unwrap();
  }

  // Read each index.json folder in each theme folder
  let mut themes = Vec::new();

  for entry in std::fs::read_dir(&theme_loc).unwrap() {
    let entry = entry.unwrap();
    let path = entry.path();

    if path.is_dir() {
      let index_path = format!("{}/index.json", path.to_str().unwrap());

      if std::path::Path::new(&index_path).exists() {
        let theme_json = std::fs::read_to_string(&index_path).unwrap();

        let mut map = HashMap::new();

        map.insert("json".to_string(), theme_json);
        map.insert("path".to_string(), path.to_str().unwrap().to_string());
        
        // Push key-value pair containing "json" and "path"
        themes.push(map);
      }
    }
  }

  return themes;
}

#[tauri::command]
// TODO: Replace with downloading the background file & saving it.
async fn get_bg_file(bg_path: String, appdata: String) -> String {
  let copy_loc = appdata;
  let query = web::query("https://api.grasscutter.io/cultivation/query").await;
  let response_data: APIQuery = match serde_json::from_str(&query) {
    Ok(data) => data,
    Err(e) => {
      println!("Failed to parse response: {}", e);
      return "".to_string();
    }
  };

  let file_name = response_data.bg_file.to_string();

  // First we see if the file already exists in our local bg folder.
  if file_helpers::dir_exists(format!("{}\\bg\\{}", copy_loc, file_name).as_str()) {
    return format!("{}\\{}", copy_loc, response_data.bg_file.as_str());
  }

  // Now we check if the bg folder, which is one directory above the game_path, exists.
  let bg_img_path = format!("{}\\{}", bg_path.clone().to_string(), file_name.as_str());

  // If it doesn't, then we do not have backgrounds to grab.
  if !file_helpers::dir_exists(&bg_path) {
    return "".to_string();
  }

  // BG folder does exist, lets see if the image exists.
  if !file_helpers::dir_exists(&bg_img_path) {
    // Image doesn't exist
    return "".to_string();
  }

  // The image exists, lets copy it to our local '\bg' folder.
  let bg_img_path_local = format!("{}\\bg\\{}", copy_loc, file_name.as_str());

  return match std::fs::copy(bg_img_path, bg_img_path_local) {
    Ok(_) => {
      // Copy was successful, lets return true.
      format!("{}\\{}", copy_loc, response_data.bg_file.as_str())
    }
    Err(e) => {
      // Copy failed, lets return false
      println!("Failed to copy background image: {}", e);
      "".to_string()
    }
  };
}

#[tauri::command]
fn base64_decode(encoded: String) -> String {
  let decoded = base64::decode(&encoded).unwrap();
  return String::from_utf8(decoded).unwrap();
}