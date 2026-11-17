use crate::AppState;
use rusqlite::params;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportRow {
    allocation_id: i64,
    promoter_name: String,
    client_name: String,
    project_name: Option<String>,
    operation_type: String,
    collection_date: String,
    statement_reference: String,
    allocation_date: String,
    allocated_minor: i64,
    frozen_minor: i64,
    reserved_minor: i64,
    executed_minor: i64,
    available_minor: i64,
}

#[tauri::command]
pub fn financial_report(
    state: State<'_, AppState>,
    promoter_id: Option<i64>,
) -> Result<Vec<FinancialReportRow>, String> {
    if promoter_id.is_some_and(|id| id <= 0) {
        return Err("معرف المرقي غير صحيح".into());
    }
    let db = state.db.lock().map_err(|_| "تعذر الوصول إلى قاعدة البيانات".to_string())?;
    let sql = "
        SELECT a.id, pr.name, cl.full_name, pj.name, a.operation_type,
               c.collection_date, e.statement_reference, a.allocation_date,
               a.amount_minor,
               COALESCE((SELECT SUM(f.amount_minor) FROM freezes f
                         WHERE f.allocation_id=a.id AND f.released_at IS NULL), 0),
               COALESCE((SELECT SUM(pd.amount_minor) FROM payment_details pd
                         JOIN outgoing_payments op ON op.id=pd.payment_id
                         WHERE pd.allocation_id=a.id AND op.status='reserved'), 0),
               COALESCE((SELECT SUM(pd.amount_minor) FROM payment_details pd
                         JOIN outgoing_payments op ON op.id=pd.payment_id
                         WHERE pd.allocation_id=a.id AND op.status='executed'), 0),
               a.amount_minor
               - COALESCE((SELECT SUM(f.amount_minor) FROM freezes f
                           WHERE f.allocation_id=a.id AND f.released_at IS NULL), 0)
               - COALESCE((SELECT SUM(pd.amount_minor) FROM payment_details pd
                           JOIN outgoing_payments op ON op.id=pd.payment_id
                           WHERE pd.allocation_id=a.id AND op.status IN ('reserved','executed')), 0)
        FROM allocations a
        JOIN promoters pr ON pr.id=a.promoter_id
        JOIN clients cl ON cl.id=a.client_id
        JOIN collections c ON c.id=a.collection_id
        JOIN bank_statement_entries e ON e.id=c.bank_entry_id
        LEFT JOIN projects pj ON pj.id=a.project_id
        WHERE (?1 IS NULL OR a.promoter_id=?1)
        ORDER BY pr.name COLLATE NOCASE, a.allocation_date DESC, a.id DESC";
    let mut stmt = db.connection.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![promoter_id], |r| Ok(FinancialReportRow {
        allocation_id: r.get(0)?,
        promoter_name: r.get(1)?,
        client_name: r.get(2)?,
        project_name: r.get(3)?,
        operation_type: r.get(4)?,
        collection_date: r.get(5)?,
        statement_reference: r.get(6)?,
        allocation_date: r.get(7)?,
        allocated_minor: r.get(8)?,
        frozen_minor: r.get(9)?,
        reserved_minor: r.get(10)?,
        executed_minor: r.get(11)?,
        available_minor: r.get(12)?,
    })).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
