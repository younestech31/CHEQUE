import {useEffect,useMemo,useState} from 'react';
import type {FormEvent} from 'react';
import {invoke} from '@tauri-apps/api/core';

type Promoter={id:number;name:string;active:boolean};
type Client={id:number;fullName:string;phone:string|null};
type Project={id:number;promoterId:number;name:string};
type Pending={id:number;collectionDate:string;statementReference:string;bankTransactionReference:string|null;amountMinor:number;allocatedMinor:number;remainingMinor:number;confirmationReference:string|null};
type Allocation={id:number;collectionId:number;collectionDate:string;statementReference:string;clientName:string;projectName:string|null;operationType:string;amountMinor:number;allocationDate:string};
const today=()=>{const d=new Date();return [d.getFullYear(),String(d.getMonth()+1).padStart(2,'0'),String(d.getDate()).padStart(2,'0')].join('-')};
const money=(n:number)=>new Intl.NumberFormat('ar-DZ',{minimumFractionDigits:2,maximumFractionDigits:2}).format(n/100)+' دج';
const toMinor=(v:string):number|null=>{const s=v.trim().replace(/\s/g,'').replace(',','.');if(!/^\d+(\.\d{1,2})?$/.test(s))return null;const [a,b='']=s.split('.');const n=Number(a)*100+Number((b+'00').slice(0,2));return Number.isSafeInteger(n)&&n>0?n:null};
export default function Classification({mode='classification'}:{mode?:'classification'|'promoters'}){
 const [promoters,setPromoters]=useState<Promoter[]>([]),[clients,setClients]=useState<Client[]>([]),[projects,setProjects]=useState<Project[]>([]);
 const [pending,setPending]=useState<Pending[]>([]),[allocations,setAllocations]=useState<Allocation[]>([]);
 const [collectionId,setCollectionId]=useState(''),[promoterId,setPromoterId]=useState(''),[clientId,setClientId]=useState(''),[projectId,setProjectId]=useState('');
 const [operation,setOperation]=useState('إيداع زبون'),[date,setDate]=useState(today()),[amount,setAmount]=useState('');
 const [clientName,setClientName]=useState(''),[clientPhone,setClientPhone]=useState(''),[showClient,setShowClient]=useState(false);
 const [projectName,setProjectName]=useState(''),[showProject,setShowProject]=useState(false);
 const [selectedPromoter,setSelectedPromoter]=useState('');
 const [error,setError]=useState(''),[success,setSuccess]=useState(''),[busy,setBusy]=useState(false);
 const refresh=async()=>{try{const [p,c,u]=await Promise.all([invoke<Promoter[]>('list_promoters'),invoke<Client[]>('list_clients'),invoke<Pending[]>('list_unclassified_collections')]);setPromoters(p);setClients(c);setPending(u)}catch(e){setError(String(e))}};
 useEffect(()=>{void refresh()},[]);
 useEffect(()=>{void invoke<Project[]>('list_projects',{promoterId:promoterId?Number(promoterId):null}).then(setProjects).catch(e=>setError(String(e)));setProjectId('')},[promoterId]);
 useEffect(()=>{if(collectionId){const c=pending.find(x=>x.id===Number(collectionId));if(c)setAmount((c.remainingMinor/100).toFixed(2))}},[collectionId,pending]);
 useEffect(()=>{if(mode==='promoters'&&selectedPromoter){void invoke<Allocation[]>('list_promoter_allocations',{promoterId:Number(selectedPromoter)}).then(setAllocations).catch(e=>setError(String(e)))}else setAllocations([])},[mode,selectedPromoter,pending]);
 const activePromoters=promoters.filter(p=>p.active);
 const chosen=pending.find(x=>x.id===Number(collectionId));
 const totalAllocated=useMemo(()=>allocations.reduce((s,a)=>s+a.amountMinor,0),[allocations]);
 const act=async(fn:()=>Promise<unknown>,message:string)=>{setBusy(true);setError('');setSuccess('');try{await fn();setSuccess(message);await refresh()}catch(e){setError(String(e))}finally{setBusy(false)}};
 const submitAllocation=async(e:FormEvent)=>{e.preventDefault();const n=toMinor(amount);if(!chosen||!promoterId||!clientId||!n){setError('اختر التحصيل والمرقي والزبون وأدخل مبلغاً صحيحاً');return}if(n>chosen.remainingMinor){setError('مبلغ التصنيف أكبر من المتبقي غير المصنف');return}await act(()=>invoke('allocate_collection',{input:{collectionId:chosen.id,promoterId:Number(promoterId),clientId:Number(clientId),projectId:projectId?Number(projectId):null,operationType:operation,allocationDate:date,amountMinor:n}}),'تم تصنيف المبلغ وربطه بالمرقي والزبون');setCollectionId('');setAmount('');};
 const addClient=async(e:FormEvent)=>{e.preventDefault();if(!clientName.trim()){setError('اسم الزبون مطلوب');return}await act(async()=>{const id=await invoke<number>('create_client',{input:{fullName:clientName,phone:clientPhone||null,notes:null}});setClientId(String(id))},'تمت إضافة الزبون');setClientName('');setClientPhone('');setShowClient(false)};
 const addProject=async(e:FormEvent)=>{e.preventDefault();if(!promoterId||!projectName.trim()){setError('اختر المرقي وأدخل اسم المشروع');return}await act(async()=>{const id=await invoke<number>('create_project',{input:{promoterId:Number(promoterId),name:projectName}});setProjectId(String(id));const ps=await invoke<Project[]>('list_projects',{promoterId:Number(promoterId)});setProjects(ps)},'تمت إضافة المشروع');setProjectName('');setShowProject(false)};
 return <div className="work-module">
 <h2>{mode==='classification'?'تصنيف المبالغ المحصلة':'قوائم المرقين العقاريين'}</h2>
 {error&&<p className="error-msg">{error}</p>}{success&&<p className="ok-msg">{success}</p>}
 {mode==='classification'?<>
 <p className="hint">هذه القائمة تعرض التحصيلات المؤكدة التي لم توزع بالكامل. لا يظهر المبلغ هنا إلا بعد إثبات دخوله فعلياً إلى الخزينة.</p>
 <div className="table-wrap"><table><thead><tr><th>رقم التحصيل</th><th>مرجع الكشف</th><th>تاريخ التحصيل</th><th>المبلغ الأصلي</th><th>المصنف</th><th>المتبقي</th></tr></thead><tbody>{pending.map(c=><tr key={c.id}><td>{c.id}</td><td>{c.statementReference}</td><td>{c.collectionDate}</td><td>{money(c.amountMinor)}</td><td>{money(c.allocatedMinor)}</td><td>{money(c.remainingMinor)}</td></tr>)}</tbody></table>{!pending.length&&<p className="empty">لا توجد مبالغ محصلة تنتظر التصنيف</p>}</div>
 <form className="entry-form" onSubmit={submitAllocation}>
 <b className="wide">إضافة سطر تصنيف</b>
 <label>التحصيل<input list="pending-list" value={collectionId} onChange={e=>setCollectionId(e.target.value)} placeholder="أدخل رقم التحصيل"/></label>
 <datalist id="pending-list">{pending.map(c=><option key={c.id} value={c.id}>{c.statementReference} — متبقي {money(c.remainingMinor)}</option>)}</datalist>
 <label>المبلغ المتبقي<input readOnly value={chosen?money(chosen.remainingMinor):'—'}/></label>
 <label>المبلغ المراد تصنيفه<input value={amount} onChange={e=>setAmount(e.target.value)} placeholder="0.00"/></label>
 <label>المرقي العقاري<select required value={promoterId} onChange={e=>setPromoterId(e.target.value)}><option value="">اختر المرقي</option>{activePromoters.map(p=><option key={p.id} value={p.id}>{p.name}</option>)}</select></label>
 <label>الزبون<select required value={clientId} onChange={e=>setClientId(e.target.value)}><option value="">اختر الزبون</option>{clients.map(c=><option key={c.id} value={c.id}>{c.fullName}</option>)}</select></label>
 <label>المشروع<select value={projectId} onChange={e=>setProjectId(e.target.value)}><option value="">بدون تحديد مشروع</option>{projects.map(p=><option key={p.id} value={p.id}>{p.name}</option>)}</select></label>
 <label>نوع العملية<input required maxLength={100} value={operation} onChange={e=>setOperation(e.target.value)}/></label>
 <label>تاريخ التصنيف<input required type="date" value={date} onChange={e=>setDate(e.target.value)}/></label>
 <button className="primary" disabled={busy}>حفظ سطر التصنيف</button>
 <button type="button" onClick={()=>setShowClient(!showClient)}>إضافة زبون جديد</button>
 <button type="button" disabled={!promoterId} onClick={()=>setShowProject(!showProject)}>إضافة مشروع</button>
 </form>
 {showClient&&<form className="entry-form deposit-box" onSubmit={addClient}><b>إضافة زبون</b><label>الاسم الكامل<input required value={clientName} onChange={e=>setClientName(e.target.value)}/></label><label>الهاتف<input value={clientPhone} onChange={e=>setClientPhone(e.target.value)}/></label><button className="primary" disabled={busy}>حفظ الزبون</button></form>}
 {showProject&&<form className="entry-form deposit-box" onSubmit={addProject}><b>إضافة مشروع للمرقي المحدد</b><label>اسم المشروع<input required value={projectName} onChange={e=>setProjectName(e.target.value)}/></label><button className="primary" disabled={busy}>حفظ المشروع</button></form>}
 </>:<>
 <p className="hint">كل سطر يعرض المبلغ المحصل والمصنف لهذا المرقي مع اسم الزبون والمشروع ونوع العملية.</p>
 <div className="entry-form"><label>اختر المرقي<select value={selectedPromoter} onChange={e=>setSelectedPromoter(e.target.value)}><option value="">اختر المرقي</option>{promoters.map(p=><option key={p.id} value={p.id}>{p.name}{p.active?'':' (غير نشط)'}</option>)}</select></label><label>إجمالي السطور المعروضة<input readOnly value={money(totalAllocated)}/></label></div>
 <div className="table-wrap"><table><thead><tr><th>تاريخ التصنيف</th><th>الزبون</th><th>المشروع</th><th>نوع العملية</th><th>مرجع كشف الخزينة</th><th>رقم التحصيل</th><th>المبلغ</th></tr></thead><tbody>{allocations.map(a=><tr key={a.id}><td>{a.allocationDate}</td><td>{a.clientName}</td><td>{a.projectName||'—'}</td><td>{a.operationType}</td><td>{a.statementReference}</td><td>{a.collectionId} / {a.collectionDate}</td><td>{money(a.amountMinor)}</td></tr>)}</tbody></table>{selectedPromoter&&!allocations.length&&<p className="empty">لا توجد مبالغ مصنفة لهذا المرقي بعد</p>}</div>
 </>}
 </div>
}
