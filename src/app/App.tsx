import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {LayoutDashboard,ReceiptText,Wallet,Tags,Building2,Snowflake,Send,FileBarChart} from 'lucide-react';
import ChequeTreasury from '../modules/treasury/ChequeTreasury';

type Page='dashboard'|'cheques'|'treasury'|'classification'|'promoters'|'freezes'|'payments'|'reports';
type Summary={treasuryBalanceMinor:number;unclassifiedMinor:number;classifiedEntitlementsMinor:number;activeFreezesMinor:number;reservedPaymentsMinor:number;executedPaymentsMinor:number;availableCashMinor:number};
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
 return <div className="layout"><aside><h2>CHEQUE</h2><small>إدارة التحصيل والخزينة</small><nav>{menu.map(({id,label,icon:Icon})=><button key={id} className={page===id?'selected':''} onClick={()=>{setPage(id);setError('')}}><Icon size={18}/>{label}</button>)}</nav><small className="offline">● يعمل محلياً على هذا الجهاز</small></aside>
 <main><header>CHEQUE　/　{title}</header><section><p className="eyebrow">إدارة مالية محلية</p><h1>{title}</h1><p>متابعة الشيكات والتحصيلات وتوزيع المستحقات والتحويلات.</p>
 {error&&<p className="error-msg">{error}</p>}
 {page==='dashboard'&&<><div className="notice"><b>مرحباً بك في CHEQUE</b><p>سجّل الشيك، ثم طابق التحصيل مع كشف الخزينة. لن يطلب منك تحديد المرقي إلا بعد تأكيد دخول المبلغ فعلياً.</p></div><div className="cards">{[['رصيد الخزينة الفعلي',summary?money(summary.treasuryBalanceMinor):'—'],['مبالغ محصلة غير مصنفة',summary?money(summary.unclassifiedMinor):'—'],['مستحقات مصنفة',summary?money(summary.classifiedEntitlementsMinor):'—'],['السيولة المتاحة',summary?money(summary.availableCashMinor):'—']].map(([k,v])=><article key={k}><small>{k}</small><strong>{v}</strong></article>)}</div><article className="panel"><b>سير العمل</b><p>استلام الشيك ← تسجيل الإيداع ← إدخال كشف الخزينة ← تأكيد التحصيل ← التصنيف ← التجميد أو التحويل.</p></article></>}
 {(page==='cheques'||page==='treasury')&&<ChequeTreasury initialTab={page}/>}
 {page!=='dashboard'&&page!=='cheques'&&page!=='treasury'&&<article className="panel"><b>{title}</b><p>هذه الوحدة ضمن خطة البناء ولم تُفعّل بعد.</p></article>}
 </section></main></div>
}
