use crate::AppState;
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use tauri::State;

fn date_ok(s:&str)->bool {
 let p:Vec<&str>=s.split('-').collect();
 if p.len()!=3||p[0].len()!=4||p[1].len()!=2||p[2].len()!=2{return false}
 let (Ok(y),Ok(m),Ok(d))=(p[0].parse::<i32>(),p[1].parse::<u32>(),p[2].parse::<u32>()) else{return false};
 if y<1900||!(1..=12).contains(&m){return false}
 let leap=y%4==0&&(y%100!=0||y%400==0);
 let max=match m{2 if leap=>29,2=>28,4|6|9|11=>30,_=>31};
 (1..=max).contains(&d)
}
fn lock_error()->String{"تعذر الوصول إلى قاعدة البيانات".into()}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct NewCheque{cheque_number:String,bank_name:Option<String>,drawer_name:Option<String>,issue_date:Option<String>,received_date:String,amount_minor:i64,notes:Option<String>}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ChequeRow{id:i64,cheque_number:String,bank_name:Option<String>,drawer_name:Option<String>,received_date:String,amount_minor:i64,deposited_minor:i64,matched_minor:i64,status:String,notes:Option<String>}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct NewStatementEntry{statement_reference:String,statement_date:String,entry_date:String,bank_transaction_reference:Option<String>,description:Option<String>,credit_minor:i64,debit_minor:i64}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct StatementEntryRow{id:i64,statement_reference:String,statement_date:String,entry_date:String,bank_transaction_reference:Option<String>,description:Option<String>,credit_minor:i64,debit_minor:i64,confirmed_minor:i64,remaining_credit_minor:i64,reconciled_debit_minor:i64,remaining_debit_minor:i64}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ChequeMatch{cheque_id:i64,matched_minor:i64}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ConfirmCollectionInput{bank_entry_id:i64,collection_date:String,amount_minor:i64,confirmation_reference:Option<String>,notes:Option<String>,cheque_matches:Vec<ChequeMatch>}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct CollectionRow{id:i64,bank_entry_id:i64,collection_date:String,amount_minor:i64,confirmation_reference:Option<String>,notes:Option<String>}

#[tauri::command]
pub fn health_check(state:State<'_,AppState>)->Result<String,String>{
 let db=state.db.lock().map_err(|_|lock_error())?;
 db.connection.query_row("SELECT 1",[],|_|Ok(())).map_err(|e|e.to_string())?;
 Ok("ok".into())
}

#[tauri::command]
pub fn create_cheque(state:State<'_,AppState>,input:NewCheque)->Result<i64,String>{
 if input.cheque_number.trim().is_empty()||input.amount_minor<=0||!date_ok(&input.received_date){return Err("رقم الشيك والمبلغ الموجب وتاريخ الاستلام الصحيح حقول إلزامية".into())}
 if input.issue_date.as_deref().is_some_and(|d|!date_ok(d)){return Err("تاريخ إصدار الشيك غير صحيح".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO cheques(cheque_number,bank_name,drawer_name,issue_date,received_date,amount_minor,notes) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![input.cheque_number.trim(),input.bank_name,input.drawer_name,input.issue_date,input.received_date,input.amount_minor,input.notes]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('cheque',?1,'received',json_object('cheque_number',?2,'amount_minor',?3))",params![id,input.cheque_number,input.amount_minor]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(id)
}

#[tauri::command]
pub fn list_cheques(state:State<'_,AppState>)->Result<Vec<ChequeRow>,String>{
 let db=state.db.lock().map_err(|_|lock_error())?;
 let mut s=db.connection.prepare("SELECT c.id,c.cheque_number,c.bank_name,c.drawer_name,c.received_date,c.amount_minor,COALESCE((SELECT SUM(amount_minor) FROM cheque_deposits d WHERE d.cheque_id=c.id),0),COALESCE((SELECT SUM(matched_minor) FROM collection_cheques m WHERE m.cheque_id=c.id),0),c.status,c.notes FROM cheques c ORDER BY c.received_date DESC,c.id DESC").map_err(|e|e.to_string())?;
 let rows=s.query_map([],|r|Ok(ChequeRow{id:r.get(0)?,cheque_number:r.get(1)?,bank_name:r.get(2)?,drawer_name:r.get(3)?,received_date:r.get(4)?,amount_minor:r.get(5)?,deposited_minor:r.get(6)?,matched_minor:r.get(7)?,status:r.get(8)?,notes:r.get(9)?})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}

#[tauri::command]
pub fn record_cheque_deposit(state:State<'_,AppState>,cheque_id:i64,deposit_date:String,amount_minor:i64,deposit_reference:Option<String>,bank_name:Option<String>,notes:Option<String>)->Result<i64,String>{
 if amount_minor<=0||!date_ok(&deposit_date){return Err("أدخل تاريخ إيداع صحيحاً ومبلغاً موجباً".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO cheque_deposits(cheque_id,deposit_date,deposit_reference,bank_name,amount_minor,notes) VALUES(?1,?2,?3,?4,?5,?6)",params![cheque_id,deposit_date,deposit_reference,bank_name,amount_minor,notes]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 tx.execute("UPDATE cheques SET status='deposited' WHERE id=?1 AND status='received'",params![cheque_id]).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('cheque_deposit',?1,'deposited',json_object('cheque_id',?2,'amount_minor',?3))",params![id,cheque_id,amount_minor]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(id)
}

#[tauri::command]
pub fn create_statement_entry(state:State<'_,AppState>,input:NewStatementEntry)->Result<i64,String>{
 if input.statement_reference.trim().is_empty()||!date_ok(&input.statement_date)||!date_ok(&input.entry_date)||input.credit_minor<0||input.debit_minor<0||((input.credit_minor==0)==(input.debit_minor==0)){return Err("يجب إدخال مرجع الكشف وتاريخين ومبلغ دائن أو مدين واحد فقط".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO bank_statement_entries(statement_reference,statement_date,entry_date,bank_transaction_reference,description,credit_minor,debit_minor) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![input.statement_reference.trim(),input.statement_date,input.entry_date,input.bank_transaction_reference,input.description,input.credit_minor,input.debit_minor]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('bank_statement_entry',?1,'imported',json_object('statement_reference',?2,'credit_minor',?3,'debit_minor',?4))",params![id,input.statement_reference,input.credit_minor,input.debit_minor]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(id)
}

#[tauri::command]
pub fn list_statement_entries(state:State<'_,AppState>)->Result<Vec<StatementEntryRow>,String>{
 let db=state.db.lock().map_err(|_|lock_error())?;
 let mut s=db.connection.prepare("SELECT e.id,e.statement_reference,e.statement_date,e.entry_date,e.bank_transaction_reference,e.description,e.credit_minor,e.debit_minor,COALESCE((SELECT SUM(amount_minor) FROM collections c WHERE c.bank_entry_id=e.id),0),e.credit_minor-COALESCE((SELECT SUM(amount_minor) FROM collections c WHERE c.bank_entry_id=e.id),0),COALESCE((SELECT SUM(r.amount_minor) FROM bank_debit_reconciliations r WHERE r.bank_entry_id=e.id),0),e.debit_minor-COALESCE((SELECT SUM(r.amount_minor) FROM bank_debit_reconciliations r WHERE r.bank_entry_id=e.id),0) FROM bank_statement_entries e ORDER BY e.entry_date DESC,e.id DESC").map_err(|e|e.to_string())?;
 let rows=s.query_map([],|r|Ok(StatementEntryRow{id:r.get(0)?,statement_reference:r.get(1)?,statement_date:r.get(2)?,entry_date:r.get(3)?,bank_transaction_reference:r.get(4)?,description:r.get(5)?,credit_minor:r.get(6)?,debit_minor:r.get(7)?,confirmed_minor:r.get(8)?,remaining_credit_minor:r.get(9)?,reconciled_debit_minor:r.get(10)?,remaining_debit_minor:r.get(11)?})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}

#[tauri::command]
pub fn confirm_collection(state:State<'_,AppState>,input:ConfirmCollectionInput)->Result<i64,String>{
 if input.amount_minor<=0||!date_ok(&input.collection_date){return Err("أدخل تاريخ تحصيل صحيحاً ومبلغاً موجباً".into())}
 let mut seen=std::collections::HashSet::new();
 if input.cheque_matches.iter().any(|m|m.cheque_id<=0||m.matched_minor<=0||!seen.insert(m.cheque_id)){return Err("مطابقة الشيكات تحتوي على مبلغ غير صحيح أو شيك مكرر".into())}
 let matched=input.cheque_matches.iter().try_fold(0_i64,|sum,m|sum.checked_add(m.matched_minor)).ok_or_else(||"تجاوز الحد الحسابي للمبلغ".to_string())?;
 if matched>input.amount_minor{return Err("مجموع مطابقة الشيكات أكبر من مبلغ التحصيل".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 let credit:Option<i64>=tx.query_row("SELECT credit_minor FROM bank_statement_entries WHERE id=?1",params![input.bank_entry_id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
 if credit.unwrap_or(0)<=0{return Err("لا يمكن تأكيد التحصيل إلا من قيد دائن موجود".into())}
 tx.execute("INSERT INTO collections(bank_entry_id,collection_date,amount_minor,confirmation_reference,notes) VALUES(?1,?2,?3,?4,?5)",params![input.bank_entry_id,input.collection_date,input.amount_minor,input.confirmation_reference,input.notes]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 for m in input.cheque_matches{
  tx.execute("INSERT INTO collection_cheques(collection_id,cheque_id,matched_minor) VALUES(?1,?2,?3)",params![id,m.cheque_id,m.matched_minor]).map_err(|e|e.to_string())?;
  tx.execute("UPDATE cheques SET status=CASE WHEN (SELECT COALESCE(SUM(matched_minor),0) FROM collection_cheques WHERE cheque_id=?1)>=amount_minor THEN 'collected' ELSE 'pending' END WHERE id=?1",params![m.cheque_id]).map_err(|e|e.to_string())?;
 }
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('collection',?1,'confirmed',json_object('bank_entry_id',?2,'amount_minor',?3,'matched_cheques_minor',?4))",params![id,input.bank_entry_id,input.amount_minor,matched]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(id)
}

#[tauri::command]
pub fn list_collections(state:State<'_,AppState>)->Result<Vec<CollectionRow>,String>{
 let db=state.db.lock().map_err(|_|lock_error())?;
 let mut s=db.connection.prepare("SELECT id,bank_entry_id,collection_date,amount_minor,confirmation_reference,notes FROM collections ORDER BY collection_date DESC,id DESC").map_err(|e|e.to_string())?;
 let rows=s.query_map([],|r|Ok(CollectionRow{id:r.get(0)?,bank_entry_id:r.get(1)?,collection_date:r.get(2)?,amount_minor:r.get(3)?,confirmation_reference:r.get(4)?,notes:r.get(5)?})).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}


#[tauri::command]
pub fn create_treasury_opening_balance(
 state:State<'_,AppState>,amount_minor:i64,balance_date:String,statement_reference:String
)->Result<(),String>{
 if amount_minor<=0||!date_ok(&balance_date)||statement_reference.trim().is_empty()||statement_reference.len()>120{
  return Err("أدخل رصيداً افتتاحياً موجباً وتاريخاً صحيحاً ومرجع كشف الحساب".into())
 }
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 let exists:i64=tx.query_row("SELECT COUNT(*) FROM treasury_opening_balance",[],|r|r.get(0)).map_err(|e|e.to_string())?;
 if exists!=0{return Err("تم تسجيل الرصيد الافتتاحي مسبقاً؛ لا يمكن تكراره".into())}
 tx.execute("INSERT INTO treasury_opening_balance(id,amount_minor,balance_date,statement_reference) VALUES(1,?1,?2,?3)",
  params![amount_minor,balance_date,statement_reference.trim()]).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('treasury','1','opening_balance',json_object('amount_minor',?1,'balance_date',?2,'statement_reference',?3))",
  params![amount_minor,balance_date,statement_reference.trim()]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(())
}

#[tauri::command]
pub fn reconcile_bank_debit_with_payment(
 state:State<'_,AppState>,bank_entry_id:i64,payment_id:i64,amount_minor:i64
)->Result<i64,String>{
 if bank_entry_id<=0||payment_id<=0||amount_minor<=0{return Err("اختر قيداً مديناً وتحويلاً منفذاً ومبلغاً موجباً".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO bank_debit_reconciliations(bank_entry_id,amount_minor,reconciliation_type,payment_id) VALUES(?1,?2,'payment',?3)",
  params![bank_entry_id,amount_minor,payment_id]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('bank_debit_reconciliation',?1,'matched_payment',json_object('bank_entry_id',?2,'payment_id',?3,'amount_minor',?4))",
  params![id,bank_entry_id,payment_id,amount_minor]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(id)
}

#[tauri::command]
pub fn reconcile_bank_debit_as_external(
 state:State<'_,AppState>,bank_entry_id:i64,amount_minor:i64
)->Result<i64,String>{
 if bank_entry_id<=0||amount_minor<=0{return Err("اختر قيداً مديناً ومبلغاً موجباً".into())}
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO bank_debit_reconciliations(bank_entry_id,amount_minor,reconciliation_type,payment_id) VALUES(?1,?2,'external',NULL)",
  params![bank_entry_id,amount_minor]).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('bank_debit_reconciliation',?1,'external_debit',json_object('bank_entry_id',?2,'amount_minor',?3))",
  params![id,bank_entry_id,amount_minor]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(id)
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct TreasuryOpeningBalanceRow{amount_minor:i64,balance_date:String,statement_reference:String}

#[tauri::command]
pub fn get_treasury_opening_balance(state:State<'_,AppState>)->Result<Option<TreasuryOpeningBalanceRow>,String>{
 let db=state.db.lock().map_err(|_|lock_error())?;
 db.connection.query_row(
  "SELECT amount_minor,balance_date,statement_reference FROM treasury_opening_balance WHERE id=1",
  [],|r|Ok(TreasuryOpeningBalanceRow{amount_minor:r.get(0)?,balance_date:r.get(1)?,statement_reference:r.get(2)?})
 ).optional().map_err(|e|e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct NewTreasuryClosingVerification{
 statement_reference:String,closing_date:String,statement_balance_minor:i64
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct TreasuryClosingVerificationRow{
 id:i64,statement_reference:String,closing_date:String,statement_balance_minor:i64,
 computed_balance_minor:i64,difference_minor:i64,pending_credit_minor:i64,pending_debit_minor:i64,
 pending_credit_count:i64,pending_debit_count:i64,status:String,created_at:String
}

#[tauri::command]
pub fn create_treasury_closing_verification(
 state:State<'_,AppState>,input:NewTreasuryClosingVerification
)->Result<i64,String>{
 let reference=input.statement_reference.trim();
 if reference.is_empty()||reference.len()>120||!date_ok(&input.closing_date)||input.statement_balance_minor<0{
  return Err("أدخل مرجع كشف الحساب وتاريخ إقفال صحيحاً ورصيداً غير سالب".into())
 }
 let mut db=state.db.lock().map_err(|_|lock_error())?;
 let tx=db.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 let opening:Option<(String,i64)>=tx.query_row(
  "SELECT balance_date,amount_minor FROM treasury_opening_balance WHERE id=1",[],
  |r|Ok((r.get(0)?,r.get(1)?))
 ).optional().map_err(|e|e.to_string())?;
 let Some((opening_date,_opening_amount))=opening else{
  return Err("يجب تثبيت الرصيد الافتتاحي قبل مطابقة رصيد الإقفال".into())
 };
 if input.closing_date<opening_date{
  return Err("تاريخ الإقفال لا يمكن أن يسبق تاريخ الرصيد الافتتاحي".into())
 }
 let statement_exists:i64=tx.query_row(
  "SELECT COUNT(*) FROM bank_statement_entries WHERE statement_reference=?1 AND entry_date<=?2",
  params![reference,input.closing_date],|r|r.get(0)
 ).map_err(|e|e.to_string())?;
 if statement_exists==0{return Err("مرجع كشف الحساب غير موجود ضمن القيود المدخلة".into())}
 let (computed,pending_credit,pending_debit,credit_count,debit_count):(i64,i64,i64,i64,i64)=tx.query_row(
  "SELECT
    COALESCE((SELECT SUM(CASE direction WHEN 'in' THEN amount_minor ELSE -amount_minor END)
      FROM treasury_ledger WHERE movement_date>=?1 AND movement_date<=?2),0),
    COALESCE((SELECT SUM(credit_minor-COALESCE((SELECT SUM(c.amount_minor) FROM collections c WHERE c.bank_entry_id=e.id),0))
      FROM bank_statement_entries e WHERE e.entry_date>=?1 AND e.entry_date<=?2
       AND e.credit_minor>COALESCE((SELECT SUM(c.amount_minor) FROM collections c WHERE c.bank_entry_id=e.id),0)),0),
    COALESCE((SELECT SUM(debit_minor-COALESCE((SELECT SUM(r.amount_minor) FROM bank_debit_reconciliations r WHERE r.bank_entry_id=e.id),0))
      FROM bank_statement_entries e WHERE e.entry_date>=?1 AND e.entry_date<=?2
       AND e.debit_minor>COALESCE((SELECT SUM(r.amount_minor) FROM bank_debit_reconciliations r WHERE r.bank_entry_id=e.id),0)),0),
    (SELECT COUNT(*) FROM bank_statement_entries e WHERE e.entry_date>=?1 AND e.entry_date<=?2
      AND e.credit_minor>COALESCE((SELECT SUM(c.amount_minor) FROM collections c WHERE c.bank_entry_id=e.id),0)),
    (SELECT COUNT(*) FROM bank_statement_entries e WHERE e.entry_date>=?1 AND e.entry_date<=?2
      AND e.debit_minor>COALESCE((SELECT SUM(r.amount_minor) FROM bank_debit_reconciliations r WHERE r.bank_entry_id=e.id),0))",
  params![opening_date,input.closing_date],
  |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))
 ).map_err(|e|e.to_string())?;
 let difference=computed.checked_sub(input.statement_balance_minor)
  .ok_or_else(||"تجاوز الحد الحسابي عند حساب فرق الإقفال".to_string())?;
 let status=if credit_count>0||debit_count>0{"pending"}else if difference==0{"matched"}else{"mismatch"};
 tx.execute(
  "INSERT INTO treasury_closing_verifications(statement_reference,closing_date,statement_balance_minor,computed_balance_minor,difference_minor,pending_credit_minor,pending_debit_minor,pending_credit_count,pending_debit_count,status)
   VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
  params![reference,input.closing_date,input.statement_balance_minor,computed,difference,pending_credit,pending_debit,credit_count,debit_count,status]
 ).map_err(|e|e.to_string())?;
 let id=tx.last_insert_rowid();
 tx.execute("INSERT INTO audit_log(entity_type,entity_id,action,after_json) VALUES('treasury_closing',?1,'verified',json_object('statement_reference',?2,'closing_date',?3,'statement_balance_minor',?4,'computed_balance_minor',?5,'difference_minor',?6,'status',?7))",
  params![id,reference,input.closing_date,input.statement_balance_minor,computed,difference,status]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(id)
}

#[tauri::command]
pub fn list_treasury_closing_verifications(
 state:State<'_,AppState>
)->Result<Vec<TreasuryClosingVerificationRow>,String>{
 let db=state.db.lock().map_err(|_|lock_error())?;
 let mut s=db.connection.prepare(
  "SELECT id,statement_reference,closing_date,statement_balance_minor,computed_balance_minor,difference_minor,pending_credit_minor,pending_debit_minor,pending_credit_count,pending_debit_count,status,created_at
   FROM treasury_closing_verifications ORDER BY closing_date DESC,id DESC"
 ).map_err(|e|e.to_string())?;
 let rows=s.query_map([],|r|Ok(TreasuryClosingVerificationRow{
  id:r.get(0)?,statement_reference:r.get(1)?,closing_date:r.get(2)?,statement_balance_minor:r.get(3)?,
  computed_balance_minor:r.get(4)?,difference_minor:r.get(5)?,pending_credit_minor:r.get(6)?,pending_debit_minor:r.get(7)?,
  pending_credit_count:r.get(8)?,pending_debit_count:r.get(9)?,status:r.get(10)?,created_at:r.get(11)?
 })).map_err(|e|e.to_string())?;
 rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}
