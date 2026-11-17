mod commands;mod database;
use database::Database;
use std::sync::Mutex;
use tauri::Manager;
pub struct AppState{pub db:Mutex<Database>}
pub fn run(){tauri::Builder::default().setup(|app|{let db=Database::open(app.handle())?;app.manage(AppState{db:Mutex::new(db)});Ok(())}).invoke_handler(tauri::generate_handler![commands::dashboard_summary]).run(tauri::generate_context!()).expect("CHEQUE startup failed");}