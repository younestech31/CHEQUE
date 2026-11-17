mod commands;
mod database;
use database::Database;
use std::sync::Mutex;
use tauri::Manager;
pub struct AppState { pub db: Mutex<Database> }
pub fn run() {
 tauri::Builder::default()
  .setup(|app| { let db=Database::open(app.handle())?; app.manage(AppState{db:Mutex::new(db)}); Ok(()) })
  .invoke_handler(tauri::generate_handler![
   commands::dashboard_summary,commands::workflow::health_check,
   commands::workflow::create_cheque,commands::workflow::list_cheques,
   commands::workflow::record_cheque_deposit,commands::workflow::create_statement_entry,
   commands::workflow::list_statement_entries,commands::workflow::confirm_collection,
   commands::workflow::list_collections,
   commands::classification::list_promoters,commands::classification::list_clients,
   commands::classification::list_projects,commands::classification::create_client,
   commands::classification::create_project,commands::classification::list_unclassified_collections,
   commands::classification::allocate_collection,commands::classification::list_promoter_allocations
  ])
  .run(tauri::generate_context!()).expect("CHEQUE startup failed");
}
