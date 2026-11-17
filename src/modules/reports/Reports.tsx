import {useEffect,useMemo,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';

type Backup={fileName:string;path:string;sizeBytes:number;createdUnix:number};
type Promoter={id:number;name:string;active:boolean};
type ReportRow={
 allocationId:number;promoterName:string;clientName:string;projectName:string|null;operationType:string;
 collectionDate:string;statementReference:string;allocationDate:string;allocatedMinor:number;
 frozenMinor:number;reservedMinor:number;executedMinor:number;availableMinor:number
};
const money=(n:number)=>new Intl.NumberFormat('ar-DZ',{minimumFractionDigits:2,maximumFractionDigits:2}).format(n/100)+' دج';
const dateLabel=(unix:number)=>new Intl.DateTimeFormat('ar-DZ',{dateStyle:'medium',timeStyle:'short'}).format(new Date(unix*1000));
const sizeLabel=(bytes:number)=>bytes<1024*1024?(bytes/1024).toFixed(1)+' KB':(bytes/(1024*1024)).toFixed(2)+' MB';
export default function Reports(){
 const [backups,setBackups]=useState<Backup[]>([]),[promoters,setPromoters]=useState<Promoter[]>([]);
 const [rows,setRows]=useState<ReportRow[]>([]),[promoterId,setPromoterId]=useState('all');
 const [busy,setBusy]=useState(false),[error,setError]=useState(''),[success,setSuccess]=useState('');
 const refreshBackups=async()=>setBackups(await invoke<Backup[]>('list_backups'));
 const refreshReport=async(id=promoterId)=>setRows(await invoke<ReportRow[]>('financial_report',{promoterId:id==='all'?null:Number(id)}));
 useEffect(()=>{void Promise.all([refreshBackups(),invoke<Promoter[]>('list_promoters').then(setPromoters),refreshReport('all')]).catch(e=>setError(String(e)))},[]);
 const totals=useMemo(()=>rows.reduce((a,r)=>({allocated:a.allocated+r.allocatedMinor,frozen:a.frozen+r.frozenMinor,reserved:a.reserved+r.reservedMinor,executed:a.executed+r.executedMinor,available:a.available+r.availableMinor}),{allocated:0,frozen:0,reserved:0,executed:0,available:0}),[rows]);
 const filter=async(value:string)=>{setPromoterId(value);setError('');try{await refreshReport(value)}catch(e){setError(String(e))}};
 const create=async()=>{setBusy(true);setError('');setSuccess('');try{const b=await invoke<Backup>('create_backup');setSuccess('تم إنشاء نسخة احتياطية وفحص سلامتها بنجاح: '+b.fileName);await refreshBackups()}catch(e){setError(String(e))}finally{setBusy(false)}};
 return <div className="work-module">
 <h2>التقرير المالي حسب المرقي</h2>
 <p className="hint">يعرض كل تخصيص مرتبطاً بالمرقي والزبون والمشروع ومصدر التحصيل. التجميد والحجز والتنفيذ حالات منفصلة؛ التنفيذ فقط هو الذي يسجل خصماً فعلياً في دفتر الخزينة.</p>
 {error&&<p className="error-msg">{error}</p>}{success&&<p className="ok-msg">{success}</p>}
 <div className="notice"><label htmlFor="report-promoter">تصفية التقرير حسب المرقي</label><select id="report-promoter" value={promoterId} onChange={e=>void filter(e.target.value)}><option value="all">جميع المرقين</option>{promoters.map(p=><option key={p.id} value={p.id}>{p.name}</option>)}</select><p className="hint">عدد أسطر التخصيص: {rows.length}</p></div>
 <div className="cards">
  <article><small>إجمالي المستحقات المصنفة</small><strong>{money(totals.allocated)}</strong></article>
  <article><small>التجميد القائم</small><strong>{money(totals.frozen)}</strong></article>
  <article><small>تحويلات محجوزة</small><strong>{money(totals.reserved)}</strong></article>
  <article><small>تحويلات منفذة</small><strong>{money(totals.executed)}</strong></article>
  <article><small>المتاح على أسطر التخصيص</small><strong>{money(totals.available)}</strong></article>
 </div>
 <div className="table-wrap"><table><thead><tr><th>المرقي</th><th>الزبون</th><th>المشروع</th><th>نوع العملية</th><th>تاريخ التحصيل</th><th>مرجع الكشف</th><th>المستحق</th><th>المجمد</th><th>محجوز</th><th>منفذ</th><th>المتاح</th></tr></thead><tbody>
 {rows.map(r=><tr key={r.allocationId}><td>{r.promoterName}</td><td>{r.clientName}</td><td>{r.projectName||'—'}</td><td>{r.operationType}</td><td>{r.collectionDate}</td><td>{r.statementReference}</td><td>{money(r.allocatedMinor)}</td><td>{money(r.frozenMinor)}</td><td>{money(r.reservedMinor)}</td><td>{money(r.executedMinor)}</td><td>{money(r.availableMinor)}</td></tr>)}
 </tbody></table>{!rows.length&&<p className="empty">لا توجد تخصيصات مطابقة لهذا التقرير</p>}</div>
 <h2>النسخ الاحتياطي المحلي</h2>
 <p className="hint">ينشئ البرنامج نسخة مستقلة من قاعدة البيانات باستخدام آلية SQLite للنسخ الحي، ثم ينفذ فحص integrity_check قبل اعتمادها. تحفظ النسخ داخل مجلد بيانات التطبيق على هذا الجهاز.</p>
 <div className="notice"><b>حماية بياناتك</b><p>أنشئ نسخة احتياطية دورياً، وانسخ ملفات النسخ إلى وسيط خارجي آمن. وجود النسخة على الجهاز نفسه لا يحمي من تلف القرص أو فقدانه.</p><button className="primary" disabled={busy} onClick={()=>void create()}>{busy?'جارٍ إنشاء النسخة…':'إنشاء نسخة احتياطية الآن'}</button></div>
 <h3>النسخ المحفوظة</h3>
 <div className="table-wrap"><table><thead><tr><th>اسم الملف</th><th>تاريخ الإنشاء</th><th>الحجم</th><th>المسار المحلي</th></tr></thead><tbody>{backups.map(b=><tr key={b.fileName}><td>{b.fileName}</td><td>{dateLabel(b.createdUnix)}</td><td>{sizeLabel(b.sizeBytes)}</td><td dir="ltr">{b.path}</td></tr>)}</tbody></table>{!backups.length&&<p className="empty">لا توجد نسخ احتياطية بعد</p>}</div>
 </div>
}
