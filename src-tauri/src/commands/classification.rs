use crate::AppState;
use rusqlite::{params, TransactionBehavior};
use serde::{Deserialize, Serialize};
use tauri::State;

fn lock_error()->String{"تعذر الوصول إلى قاعدة البيانات".into()}
fn clean_required(value:&str)->bool{!value.trim().is_empty()}
fn date_ok(s:&str)->bool {
 let p:Vec<&str>=s.split('-').collect();
 if p.len()!=3||p[0].len()!=4||p[1].len()!=2||p[2].len()!=2{return false}
 let (Ok(y),Ok(m),Ok(d))=(p[0].parse::<i32>(),p[1].parse::<u32>(),p[2].parse::<u32>()) else{return false};
 if y<1900||!(1..=12).contains(&m){return false}
 let leap=y%4==0&&(y%100!=0||y%400==0);
 let max=match m{2 if leap=>29,2=>28,4|6|9|11=>30,_=>31};
 (1..=max).contains(&d)
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct PromoterRow{id:i64,name:String,active:bool}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ClientRow{id:i64,full_name:String,phone:Option<String>}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ProjectRow{id:i64,promoter_id:i64,name:String}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct UnclassifiedCollectionRow{
 id:i64,collection_date:String,statement_reference:String,bank_transaction_reference:Option<String>,
 amount_minor:i64,allocated_minor:i64,remaining_minor:i64,confirmation_reference:Option<String>
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct AllocationRow{
 id:i64,collection_id:i64,collection_date:String,statement_reference:String,client_name:String,project_name:Option<String>,
 operation_type:String,amount_minor:i64,allocation_date:String
}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct NewClient{full_name:String,phone:Option<String>,notes:Option<String>}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct NewProject{promoter_id:i64,name:String}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct NewAllocation{
 collection_id:i64,promoter_id:i64,client_id:i64,project_id:Option<i64>,
 operation_type:String,allocation_date:String,amount_minor:i64
}

#[tauri::command]
pub fn list_promoters(state:State<'_,AppState>)->Result<Vec<PromoterRow>,String>{
 let db=state.db.lock().map_err(|_|lock_error())?;
 let mut s=db.connection.prepare("SELECT id,name,active FROM promoters ORDER BY name COLLATE NOCASE").map_err(|e|e.to_string())?;
 let rows=s.query_map([],|r|Ok(PromoterRow{id:r.get(0)?,name:r.get(1)?,active:r.get::<_,i64>(2)?==1})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}
#[tauri::command]
pub fn list_clients(state:State<'_,AppState>)->Result<Vec<ClientRow>,String>{
 let db=state.db.lock().map_err(|_|lock_error())?;
 let mut s=db.connection.prepare("SELECT id,full_name,phone FROM clients ORDER BY full_name COLLATE NOCASE,id").map_err(|e|e.to_string())?;
 let rows=s.query_map([],|r|Ok(ClientRow{id:r.get(0)?,full_name:r.get(1)?,phone:r.get(2)?})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}
#[tauri::command]
pub fn list_projects(state:State<'_,AppState>,promoter_id:Option<i64>)->Result<Vec<ProjectRow>,String>{
 let db=state.db.lock().map_err(|_|lock_error())?;
 let mut s=db.connection.prepare("SELECT id,promoter_id,name FROM projects WHERE (?1 IS NULL OR promoter_id=?1) ORDER BY name COLLATE NOCASE").map_err(|e|e.to_string())?;
 let rows=s.query_map(params![promoter_id],|r|Ok(ProjectRow{id:r.get(0)?,promoter_id:r.get(1)?,name:r.get(2)?})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}
#[tauri::command]
pub fn create_client(state:State<'_,AppState>,input:NewClient)->Result<i64,String>{
 if !clean_required(&input.full_name){return Err("اسم الزبون مطلوب".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO clients(full_name,phone,notes) VALUES(?1,?2,?3)",params![input.full_name.trim(),input.phone,input.notes]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('client',?1,'created',json_object('full_name',?2))",params![id,input.full_name.trim()]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(id)
}
#[tauri::command]
pub fn create_project(state:State<'_,AppState>,input:NewProject)->Result<i64,String>{
 if input.promoter_id<=0||!clean_required(&input.name){return Err("اختر مرقياً وأدخل اسم المشروع".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 let active:i64=tx.query_row("SELECT active FROM promoters WHERE id=?1",params![input.promoter_id],|r|r.get(0)).map_err(|e|e.to_string())?;
 if active!=1{return Err("المرقي غير نشط".into())}
 tx.execute("INSERT INTO projects(promoter_id,name) VALUES(?1,?2)",params![input.promoter_id,input.name.trim()]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('project',?1,'created',json_object('promoter_id',?2,'name',?3))",params![id,input.promoter_id,input.name.trim()]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(id)
}
#[tauri::command]
pub fn list_unclassified_collections(state:State<'_,AppState>)->Result<Vec<UnclassifiedCollectionRow>,String>{
 let db=state.db.lock().map_err(|_|lock_error())?;
 let mut s=db.connection.prepare("SELECT c.id,c.collection_date,e.statement_reference,e.bank_transaction_reference,c.amount_minor,COALESCE(SUM(a.amount_minor),0),c.amount_minor-COALESCE(SUM(a.amount_minor),0),c.confirmation_reference FROM collections c JOIN bank_statement_entries e ON e.id=c.bank_entry_id LEFT JOIN allocations a ON a.collection_id=c.id GROUP BY c.id HAVING c.amount_minor>COALESCE(SUM(a.amount_minor),0) ORDER BY c.collection_date,c.id").map_err(|e|e.to_string())?;
 let rows=s.query_map([],|r|Ok(UnclassifiedCollectionRow{id:r.get(0)?,collection_date:r.get(1)?,statement_reference:r.get(2)?,bank_transaction_reference:r.get(3)?,amount_minor:r.get(4)?,allocated_minor:r.get(5)?,remaining_minor:r.get(6)?,confirmation_reference:r.get(7)?})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}
#[tauri::command]
pub fn allocate_collection(state:State<'_,AppState>,input:NewAllocation)->Result<i64,String>{
 if input.collection_id<=0||input.promoter_id<=0||input.client_id<=0||input.amount_minor<=0||!clean_required(&input.operation_type)||input.operation_type.trim().len()>100||!date_ok(&input.allocation_date){return Err("تحقق من الزبون والمرقي والتاريخ ونوع العملية والمبلغ".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO allocations(collection_id,promoter_id,client_id,project_id,allocation_date,amount_minor,operation_type) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![input.collection_id,input.promoter_id,input.client_id,input.project_id,input.allocation_date,input.amount_minor,input.operation_type.trim()]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('allocation',?1,'classified',json_object('collection_id',?2,'promoter_id',?3,'client_id',?4,'project_id',?5,'amount_minor',?6,'operation_type',?7))",params![id,input.collection_id,input.promoter_id,input.client_id,input.project_id,input.amount_minor,input.operation_type.trim()]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(id)
}
#[tauri::command]
pub fn list_promoter_allocations(state:State<'_,AppState>,promoter_id:i64)->Result<Vec<AllocationRow>,String>{
 if promoter_id<=0{return Err("معرف المرقي غير صحيح".into())}
 let db=state.db.lock().map_err(|_|lock_error())?;
 let mut s=db.connection.prepare("SELECT a.id,a.collection_id,c.collection_date,e.statement_reference,cl.full_name,p.name,a.operation_type,a.amount_minor,a.allocation_date FROM allocations a JOIN collections c ON c.id=a.collection_id JOIN bank_statement_entries e ON e.id=c.bank_entry_id JOIN clients cl ON cl.id=a.client_id LEFT JOIN projects p ON p.id=a.project_id WHERE a.promoter_id=?1 ORDER BY a.allocation_date DESC,a.id DESC").map_err(|e|e.to_string())?;
 let rows=s.query_map(params![promoter_id],|r|Ok(AllocationRow{id:r.get(0)?,collection_id:r.get(1)?,collection_date:r.get(2)?,statement_reference:r.get(3)?,client_name:r.get(4)?,project_name:r.get(5)?,operation_type:r.get(6)?,amount_minor:r.get(7)?,allocation_date:r.get(8)?})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}
