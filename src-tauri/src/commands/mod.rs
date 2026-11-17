pub mod workflow;
pub mod classification;
pub mod payments;
pub mod maintenance;
use crate::AppState;
use rusqlite::params;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct DashboardSummary {
 pub treasury_balance_minor:i64,pub unclassified_minor:i64,pub classified_entitlements_minor:i64,
 pub active_freezes_minor:i64,pub reserved_payments_minor:i64,pub executed_payments_minor:i64,pub available_cash_minor:i64
}
#[tauri::command]
pub fn dashboard_summary(state:State<'_,AppState>)->Result<DashboardSummary,String>{
 let db=state.db.lock().map_err(|_|"تعذر الوصول إلى قاعدة البيانات".to_string())?;
 let c=&db.connection;
 let q=|sql:&str|c.query_row(sql,params![],|r|r.get::<_,i64>(0)).map_err(|e|e.to_string());
 let balance=q("SELECT COALESCE(SUM(CASE direction WHEN 'in' THEN amount_minor ELSE -amount_minor END),0) FROM treasury_ledger")?;
 let unclassified=q("SELECT COALESCE(SUM(c.amount_minor-COALESCE(a.total,0)),0) FROM collections c LEFT JOIN (SELECT collection_id,SUM(amount_minor) total FROM allocations GROUP BY collection_id) a ON a.collection_id=c.id")?;
 let entitled=q("SELECT COALESCE(SUM(amount_minor),0) FROM allocations")?;
 let frozen=q("SELECT COALESCE(SUM(amount_minor),0) FROM freezes WHERE released_at IS NULL")?;
 let reserved=q("SELECT COALESCE(SUM(pd.amount_minor),0) FROM payment_details pd JOIN outgoing_payments p ON p.id=pd.payment_id WHERE p.status='reserved'")?;
 let executed=q("SELECT COALESCE(SUM(pd.amount_minor),0) FROM payment_details pd JOIN outgoing_payments p ON p.id=pd.payment_id WHERE p.status='executed'")?;
 Ok(DashboardSummary{treasury_balance_minor:balance,unclassified_minor:unclassified,classified_entitlements_minor:entitled,active_freezes_minor:frozen,reserved_payments_minor:reserved,executed_payments_minor:executed,available_cash_minor:balance-reserved})
}
