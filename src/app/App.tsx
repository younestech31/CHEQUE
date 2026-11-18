import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {LayoutDashboard,ReceiptText,Wallet,Tags,Building2,Snowflake,Send,FileBarChart} from 'lucide-react';
import ChequeTreasury from '../modules/treasury/ChequeTreasury';
import Classification from '../modules/classification/Classification';
import PaymentManager from '../modules/payments/PaymentManager';
import Reports from '../modules/reports/Reports';

type Page='dashboard'|'cheques'|'treasury'|'classification'|'promoters'|'freezes'|'payments'|'reports';
type Summary={treasuryBalanceMinor:number;unclassifiedMinor:number;classifiedEntitlementsMinor:number;activeFreezesMinor:number;reservedPaymentsMinor:number;executedPaymentsMinor:number;availableCashMinor:number;openingBalanceSet:boolean};
const menu:{id:Page;label:string;icon:typeof LayoutDashboard}[]=[
 {id:'dashboard',label:'لوحة القيادة',icon:LayoutDashboard},{id:'cheques',label:'الشيكات الواردة',icon:ReceiptText},
 {id:'treasury',label:'الخزينة والتحصيل',icon:Wallet},{id:'classification',label:'تصنيف المبالغ',icon:Tags},
 {id:'promoters',label:'قوائم المرقين',icon:Building2},{id:'freezes',label:'التجميد',icon:Snowflake},
 {id:'payments',label:'التحويلات',icon:Send},{id:'reports',label:'التقارير',icon:FileBarChart}
];
const money=(n:number)=>new Intl.NumberFormat('ar-DZ',{minimumFractionDigits:2,maximumFractionDigits:2}).format(n/100)+' دج';
export default function App(){
 const [page,setPage]=useState<Page>('dashboard'),[summary,setSummary]=useState<Summary|null>(null),[error,setError]=useState('');
 const refresh=()=>invoke<Summary>('dashboard_summary').then(setSummary).catch(e=>setError(String(e)));
 useEffect(()=>{void refresh()},[page]);
 const title=menu.find(x=>x.id===page)?.label;
 return <div className="layout"><aside><h2>CHEQUE</h2><small>إدارة التحصيل والخزينة</small><nav aria-label="التنقل الرئيسي">{menu.map(({id,label,icon:Icon})=><button key={id} type="button" aria-current={page===id?'page':undefined} className={page===id?'selected':''} onClick={()=>{setPage(id);setError('')}}><Icon size={18} aria-hidden="true"/><span>{label}</span></button>)}</nav><small className="offline">● يعمل محلياً على هذا الجهاز</small></aside>
 <main><header>CHEQUE　/　{title}</header><section><p className="eyebrow">إدارة مالية محلية</p><h1>{title}</h1><p>متابعة الشيكات والتحصيلات وتوزيع المستحقات والتحويلات.</p>
 {error&&<p className="error-msg" role="alert">{error}</p>}
 {page==='dashboard'&&<><div className="notice"><b>مرحباً بك في CHEQUE</b><p>سجّل الشيك، ثم طابق التحصيل مع كشف الخزينة. لن يطلب منك تحديد المرقي إلا بعد تأكيد دخول المبلغ فعلياً.</p></div><div className="cards">{[['رصيد الخزينة بعد التسوية',summary?money(summary.treasuryBalanceMinor):'—'],['مبالغ محصلة غير مصنفة',summary?money(summary.unclassifiedMinor):'—'],['مستحقات مصنفة',summary?money(summary.classifiedEntitlementsMinor):'—'],['السيولة المتاحة',summary?money(summary.availableCashMinor):'—'],['مبالغ مجمدة',summary?money(summary.activeFreezesMinor):'—'],['تحويلات محجوزة',summary?money(summary.reservedPaymentsMinor):'—'],['تحويلات منفذة',summary?money(summary.executedPaymentsMinor):'—']].map(([k,v])=><article key={k}><small>{k}</small><strong>{v}</strong></article>)}</div>{summary&&!summary.openingBalanceSet&&<article className="panel"><b>تنبيه: لم يثبت الرصيد الافتتاحي</b><p>رصيد الخزينة المعروض غير معتمد للمطابقة النهائية. انتقل إلى الخزينة والتحصيل وسجّل رصيد بداية يوم كشف الحساب قبل إجراء الحجوزات.</p></article>}<article className="panel"><b>سير العمل</b><p>استلام الشيك ← تسجيل الإيداع ← إدخال كشف الخزينة ← تأكيد التحصيل ← التصنيف ← التجميد أو التحويل.</p></article></>}
 {(page==='cheques'||page==='treasury')&&<ChequeTreasury initialTab={page}/>}
 {page==='classification'&&<Classification mode='classification'/>}
 {page==='promoters'&&<Classification mode='promoters'/>}
 {page==='payments'&&<PaymentManager mode='payments'/>}
 {page==='freezes'&&<PaymentManager mode='freezes'/>}
 {page==='reports'&&<Reports/>}
 </section></main></div>
}
