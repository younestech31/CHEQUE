use crate::AppState;
use rusqlite::{backup::Backup, Connection};
use serde::Serialize;
use std::{fs, path::PathBuf, time::{Duration,SystemTime,UNIX_EPOCH}};
use tauri::{AppHandle,Manager,State};

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct BackupRow{file_name:String,path:String,size_bytes:u64,created_unix:i64}

fn backup_dir(app:&AppHandle)->Result<PathBuf,String>{
 let mut path=app.path().app_data_dir().map_err(|e|e.to_string())?;
 path.push("backups");
 fs::create_dir_all(&path).map_err(|e|e.to_string())?;
 Ok(path)
}

#[tauri::command]
pub fn create_backup(app:AppHandle,state:State<'_,AppState>)->Result<BackupRow,String>{
 let db=state.db.lock().map_err(|_|"تعذر الوصول إلى قاعدة البيانات".to_string())?;
 let dir=backup_dir(&app)?;
 let stamp=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?;
 let file_name=format!("cheque-backup-{}.sqlite3",stamp.as_nanos());
 let path=dir.join(&file_name);
 let mut destination=Connection::open(&path).map_err(|e|e.to_string())?;
 let result=(||{
  let backup=Backup::new(&db.connection,&mut destination).map_err(|e|e.to_string())?;
  backup.run_to_completion(64,Duration::from_millis(50),None).map_err(|e|e.to_string())?;
  drop(backup);
  let integrity:String=destination.query_row("PRAGMA integrity_check",[],|r|r.get(0)).map_err(|e|e.to_string())?;
  if integrity!="ok"{return Err(format!("فشل فحص سلامة النسخة: {integrity}"))}
  Ok(())
 })();
 if let Err(error)=result{drop(destination);let _=fs::remove_file(&path);return Err(error)}
 let metadata=fs::metadata(&path).map_err(|e|e.to_string())?;
 let created_unix=stamp.as_secs().min(i64::MAX as u64) as i64;
 Ok(BackupRow{file_name,path:path.to_string_lossy().into_owned(),size_bytes:metadata.len(),created_unix})
}

#[tauri::command]
pub fn list_backups(app:AppHandle)->Result<Vec<BackupRow>,String>{
 let dir=backup_dir(&app)?;
 let mut items=Vec::new();
 for entry in fs::read_dir(dir).map_err(|e|e.to_string())?{
  let entry=entry.map_err(|e|e.to_string())?;
  let path=entry.path();
  let file_name=path.file_name().and_then(|x|x.to_str()).unwrap_or("");
  if path.extension().and_then(|x|x.to_str())!=Some("sqlite3") || !file_name.starts_with("cheque-backup-"){continue}
  let metadata=entry.metadata().map_err(|e|e.to_string())?;
  if !metadata.is_file(){continue}
  let created_unix=metadata.modified().ok().and_then(|t|t.duration_since(UNIX_EPOCH).ok()).map(|d|d.as_secs().min(i64::MAX as u64) as i64).unwrap_or(0);
  items.push(BackupRow{file_name:path.file_name().and_then(|x|x.to_str()).unwrap_or("backup.sqlite3").to_string(),path:path.to_string_lossy().into_owned(),size_bytes:metadata.len(),created_unix});
 }
 items.sort_by(|a,b|b.created_unix.cmp(&a.created_unix));
 Ok(items)
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct BackupVerification { file_name:String, integrity:String, foreign_key_violations:usize }

#[tauri::command]
pub fn verify_backup(app:AppHandle,file_name:String)->Result<BackupVerification,String>{
 use rusqlite::OpenFlags;
 use std::path::Path;
 if file_name.is_empty() || file_name!=Path::new(&file_name).file_name().and_then(|x|x.to_str()).unwrap_or("") || !file_name.starts_with("cheque-backup-") || !file_name.ends_with(".sqlite3") { return Err("اسم ملف النسخة الاحتياطية غير صالح".into()); }
 let dir=backup_dir(&app)?;
 let canonical_dir=dir.canonicalize().map_err(|e|e.to_string())?;
 let canonical_file=dir.join(&file_name).canonicalize().map_err(|e|format!("تعذر العثور على النسخة: {e}"))?;
 if canonical_file.parent()!=Some(canonical_dir.as_path()) || !canonical_file.is_file() { return Err("مسار النسخة الاحتياطية خارج مجلد النسخ المعتمد".into()); }
 let connection=Connection::open_with_flags(&canonical_file,OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e|format!("تعذر فتح النسخة للقراءة فقط: {e}"))?;
 let required_tables:i64=connection.query_row(
  "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('schema_migrations','cheques','bank_statement_entries','collections','allocations','outgoing_payments','payment_details','treasury_ledger','treasury_opening_balance','bank_debit_reconciliations','treasury_closing_verifications')",
  [],|r|r.get(0)
 ).map_err(|e|e.to_string())?;
 if required_tables!=11 { return Err("الملف ليس نسخة مكتملة من قاعدة بيانات CHEQUE".into()); }
 let migration_count:i64=connection.query_row(
  "SELECT COUNT(*) FROM schema_migrations WHERE version IN (2,3,4,5,6,7,8)",[],|r|r.get(0)
 ).map_err(|e|format!("سجل ترحيلات قاعدة البيانات غير صالح: {e}"))?;
 if migration_count!=7 { return Err("نسخة قاعدة البيانات لا تحتوي على جميع التحديثات المطلوبة".into()); }
 let integrity:String=connection.query_row("PRAGMA integrity_check",[],|r|r.get(0)).map_err(|e|e.to_string())?;
 if integrity!="ok" { return Err(format!("فشل فحص سلامة SQLite: {integrity}")); }
 let mut stmt=connection.prepare("PRAGMA foreign_key_check").map_err(|e|e.to_string())?;
 let mut rows=stmt.query([]).map_err(|e|e.to_string())?;
 let mut violations=0usize;
 while rows.next().map_err(|e|e.to_string())?.is_some() { violations+=1; }
 if violations>0 { return Err(format!("النسخة تحتوي على {violations} مخالفة للمفاتيح الخارجية")); }
 Ok(BackupVerification{file_name,integrity,foreign_key_violations:violations})
}
