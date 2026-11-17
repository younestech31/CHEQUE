use crate::AppState;
use rusqlite::{params, TransactionBehavior};
use serde::{Deserialize, Serialize};
use tauri::State;

fn lock_error()->String{"تعذر الوصول إلى قاعدة البيانات".into()}
fn date_ok(s:&str)->bool{
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
pub struct EligibleAllocation{
 allocation_id:i64,client_name:String,project_name:Option<String>,operation_type:String,
 allocation_date:String,collection_date:String,statement_reference:String,
 allocation_minor:i64,frozen_minor:i64,committed_minor:i64,available_minor:i64
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct FreezeRow{
 id:i64,allocation_id:i64,promoter_id:i64,client_name:String,project_name:Option<String>,
 amount_minor:i64,reason:String,frozen_at:String,released_at:Option<String>,release_note:Option<String>
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct PaymentRow{
 id:i64,payment_reference:String,payment_type:String,promoter_id:i64,promoter_name:String,
 preparation_date:String,status:String,total_minor:i64,execution_date:Option<String>,
 external_reference:Option<String>,notes:Option<String>
}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct PaymentDetailInput{allocation_id:i64,amount_minor:i64}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct NewPayment{
 payment_reference:String,payment_type:String,promoter_id:i64,preparation_date:String,
 details:Vec<PaymentDetailInput>
}

#[tauri::command]
pub fn list_eligible_allocations(state:State<'_,AppState>,promoter_id:i64)->Result<Vec<EligibleAllocation>,String>{
 if promoter_id<=0{return Err("اختر مرقياً صحيحاً".into())}
 let db=state.db.lock().map_err(|_|lock_error())?;
 let sql="SELECT a.id,cl.full_name,pr.name,a.operation_type,a.allocation_date,c.collection_date,e.statement_reference,a.amount_minor,
 COALESCE((SELECT SUM(f.amount_minor) FROM freezes f WHERE f.allocation_id=a.id AND f.released_at IS NULL),0),
 COALESCE((SELECT SUM(pd.amount_minor) FROM payment_details pd JOIN outgoing_payments op ON op.id=pd.payment_id WHERE pd.allocation_id=a.id AND op.status IN('reserved','executed')),0),
 a.amount_minor
 -COALESCE((SELECT SUM(f.amount_minor) FROM freezes f WHERE f.allocation_id=a.id AND f.released_at IS NULL),0)
 -COALESCE((SELECT SUM(pd.amount_minor) FROM payment_details pd JOIN outgoing_payments op ON op.id=pd.payment_id WHERE pd.allocation_id=a.id AND op.status IN('reserved','executed')),0)
 FROM allocations a JOIN clients cl ON cl.id=a.client_id LEFT JOIN projects pr ON pr.id=a.project_id
 JOIN collections c ON c.id=a.collection_id JOIN bank_statement_entries e ON e.id=c.bank_entry_id
 WHERE a.promoter_id=?1 AND a.amount_minor
 -COALESCE((SELECT SUM(f.amount_minor) FROM freezes f WHERE f.allocation_id=a.id AND f.released_at IS NULL),0)
 -COALESCE((SELECT SUM(pd.amount_minor) FROM payment_details pd JOIN outgoing_payments op ON op.id=pd.payment_id WHERE pd.allocation_id=a.id AND op.status IN('reserved','executed')),0)>0
 ORDER BY a.allocation_date,a.id";
 let mut s=db.connection.prepare(sql).map_err(|e|e.to_string())?;
 let rows=s.query_map(params![promoter_id],|r|Ok(EligibleAllocation{
  allocation_id:r.get(0)?,client_name:r.get(1)?,project_name:r.get(2)?,operation_type:r.get(3)?,
  allocation_date:r.get(4)?,collection_date:r.get(5)?,statement_reference:r.get(6)?,
  allocation_minor:r.get(7)?,frozen_minor:r.get(8)?,committed_minor:r.get(9)?,available_minor:r.get(10)?
 })).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}

#[tauri::command]
pub fn list_freezes(state:State<'_,AppState>,promoter_id:Option<i64>)->Result<Vec<FreezeRow>,String>{
 let db=state.db.lock().map_err(|_|lock_error())?;
 let mut s=db.connection.prepare("SELECT f.id,f.allocation_id,a.promoter_id,cl.full_name,pr.name,f.amount_minor,f.reason,f.frozen_at,f.released_at,f.release_note FROM freezes f JOIN allocations a ON a.id=f.allocation_id JOIN clients cl ON cl.id=a.client_id LEFT JOIN projects pr ON pr.id=a.project_id WHERE (?1 IS NULL OR a.promoter_id=?1) ORDER BY f.frozen_at DESC,f.id DESC").map_err(|e|e.to_string())?;
 let rows=s.query_map(params![promoter_id],|r|Ok(FreezeRow{id:r.get(0)?,allocation_id:r.get(1)?,promoter_id:r.get(2)?,client_name:r.get(3)?,project_name:r.get(4)?,amount_minor:r.get(5)?,reason:r.get(6)?,frozen_at:r.get(7)?,released_at:r.get(8)?,release_note:r.get(9)?})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}

#[tauri::command]
pub fn create_freeze(state:State<'_,AppState>,allocation_id:i64,amount_minor:i64,reason:String,frozen_at:String)->Result<i64,String>{
 if allocation_id<=0||amount_minor<=0||reason.trim().is_empty()||!date_ok(&frozen_at){return Err("أدخل سطراً ومبلغاً وسبباً وتاريخ تجميد صحيحاً".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO freezes(allocation_id,amount_minor,reason,frozen_at) VALUES(?1,?2,?3,?4)",params![allocation_id,amount_minor,reason.trim(),frozen_at]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('freeze',?1,'frozen',json_object('allocation_id',?2,'amount_minor',?3,'reason',?4))",params![id,allocation_id,amount_minor,reason.trim()]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(id)
}

#[tauri::command]
pub fn release_freeze(state:State<'_,AppState>,freeze_id:i64,released_at:String,release_note:String)->Result<(),String>{
 if freeze_id<=0||!date_ok(&released_at)||release_note.trim().is_empty(){return Err("أدخل تاريخ رفع التجميد وسببه".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 let frozen_at:String=tx.query_row("SELECT frozen_at FROM freezes WHERE id=?1 AND released_at IS NULL",params![freeze_id],|r|r.get(0)).map_err(|e|e.to_string())?;
 if released_at<frozen_at{return Err("تاريخ رفع التجميد لا يمكن أن يسبق تاريخ التجميد".into())}
 let changed=tx.execute("UPDATE freezes SET released_at=?1,release_note=?2 WHERE id=?3 AND released_at IS NULL",params![released_at,release_note.trim(),freeze_id]).map_err(|e|e.to_string())?;
 if changed!=1{return Err("التجميد غير موجود أو سبق رفعه".into())}
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('freeze',?1,'released',json_object('released_at',?2,'release_note',?3))",params![freeze_id,released_at,release_note.trim()]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(())
}

#[tauri::command]
pub fn create_payment(state:State<'_,AppState>,input:NewPayment)->Result<i64,String>{
 if input.payment_reference.trim().is_empty()||input.payment_reference.len()>80||input.promoter_id<=0||!date_ok(&input.preparation_date)||!matches!(input.payment_type.as_str(),"periodic"|"exceptional")||input.details.is_empty(){return Err("تحقق من مرجع التحويل ونوعه والمرقي والتاريخ والأسطر".into())}
 let mut seen=std::collections::HashSet::new();
 let total=input.details.iter().try_fold(0_i64,|sum,d|{
  if d.allocation_id<=0||d.amount_minor<=0||!seen.insert(d.allocation_id){return None}
  sum.checked_add(d.amount_minor)
 }).ok_or_else(||"توجد أسطر مكررة أو مبالغ غير صحيحة أو تجاوز حسابي".to_string())?;
 if total<=0{return Err("مجموع التحويل يجب أن يكون موجباً".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO outgoing_payments(payment_reference,payment_type,promoter_id,preparation_date,status) VALUES(?1,?2,?3,?4,'prepared')",params![input.payment_reference.trim(),input.payment_type,input.promoter_id,input.preparation_date]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 for detail in input.details{
  let available:i64=tx.query_row(
   "SELECT a.amount_minor
    -COALESCE((SELECT SUM(f.amount_minor) FROM freezes f WHERE f.allocation_id=a.id AND f.released_at IS NULL),0)
    -COALESCE((SELECT SUM(pd.amount_minor) FROM payment_details pd JOIN outgoing_payments op ON op.id=pd.payment_id WHERE pd.allocation_id=a.id AND op.status IN('reserved','executed')),0)
    FROM allocations a WHERE a.id=?1 AND a.promoter_id=?2",
   params![detail.allocation_id,input.promoter_id],|r|r.get(0)
  ).map_err(|_|"سطر الاستحقاق غير موجود أو لا يتبع المرقي المحدد".to_string())?;
  if detail.amount_minor>available{return Err("مبلغ التحويل يتجاوز المستحق المتاح بعد التجميد والحجوزات السابقة".into())}
  tx.execute("INSERT INTO payment_details(payment_id,allocation_id,amount_minor) VALUES(?1,?2,?3)",params![id,detail.allocation_id,detail.amount_minor]).map_err(|e|e.to_string())?;
 }
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('payment',?1,'prepared',json_object('payment_reference',?2,'promoter_id',?3,'total_minor',?4))",params![id,input.payment_reference.trim(),input.promoter_id,total]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(id)
}

#[tauri::command]
pub fn reserve_payment(state:State<'_,AppState>,payment_id:i64)->Result<(),String>{
 if payment_id<=0{return Err("معرف التحويل غير صحيح".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 let changed=tx.execute("UPDATE outgoing_payments SET status='reserved' WHERE id=?1 AND status='prepared'",params![payment_id]).map_err(|e|e.to_string())?;
 if changed!=1{return Err("التحويل غير موجود أو ليس في حالة التحضير".into())}
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action) VALUES('payment',?1,'reserved')",params![payment_id]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(())
}

#[tauri::command]
pub fn execute_payment(state:State<'_,AppState>,payment_id:i64,execution_date:String,external_reference:Option<String>,notes:Option<String>)->Result<(),String>{
 if payment_id<=0||!date_ok(&execution_date){return Err("أدخل تاريخ تنفيذ صحيحاً".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 let changed=tx.execute("UPDATE outgoing_payments SET status='executed',execution_date=?1,external_reference=?2,notes=?3 WHERE id=?4 AND status='reserved'",params![execution_date,external_reference,notes,payment_id]).map_err(|e|e.to_string())?;
 if changed!=1{return Err("لا يمكن تسجيل التنفيذ إلا لتحويل محجوز".into())}
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('payment',?1,'executed',json_object('execution_date',?2,'external_reference',?3))",params![payment_id,execution_date,external_reference]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(())
}

#[tauri::command]
pub fn cancel_payment(state:State<'_,AppState>,payment_id:i64)->Result<(),String>{
 if payment_id<=0{return Err("معرف التحويل غير صحيح".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 let changed=tx.execute("UPDATE outgoing_payments SET status='cancelled' WHERE id=?1 AND status IN('prepared','reserved')",params![payment_id]).map_err(|e|e.to_string())?;
 if changed!=1{return Err("لا يمكن إلغاء تحويل منفذ أو غير موجود".into())}
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action) VALUES('payment',?1,'cancelled')",params![payment_id]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(())
}

#[tauri::command]
pub fn list_payments(state:State<'_,AppState>)->Result<Vec<PaymentRow>,String>{
 let db=state.db.lock().map_err(|_|lock_error())?;
 let mut s=db.connection.prepare("SELECT p.id,p.payment_reference,p.payment_type,p.promoter_id,pr.name,p.preparation_date,p.status,COALESCE(SUM(pd.amount_minor),0),p.execution_date,p.external_reference,p.notes FROM outgoing_payments p JOIN promoters pr ON pr.id=p.promoter_id LEFT JOIN payment_details pd ON pd.payment_id=p.id GROUP BY p.id ORDER BY p.preparation_date DESC,p.id DESC").map_err(|e|e.to_string())?;
 let rows=s.query_map([],|r|Ok(PaymentRow{id:r.get(0)?,payment_reference:r.get(1)?,payment_type:r.get(2)?,promoter_id:r.get(3)?,promoter_name:r.get(4)?,preparation_date:r.get(5)?,status:r.get(6)?,total_minor:r.get(7)?,execution_date:r.get(8)?,external_reference:r.get(9)?,notes:r.get(10)?})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}
