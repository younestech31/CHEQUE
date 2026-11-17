import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';

type Backup={fileName:string;path:string;sizeBytes:number;createdUnix:number};
const dateLabel=(unix:number)=>new Intl.DateTimeFormat('ar-DZ',{dateStyle:'medium',timeStyle:'short'}).format(new Date(unix*1000));
const sizeLabel=(bytes:number)=>bytes<1024*1024?(bytes/1024).toFixed(1)+' KB':(bytes/(1024*1024)).toFixed(2)+' MB';
export default function Reports(){
 const [backups,setBackups]=useState<Backup[]>([]),[busy,setBusy]=useState(false),[error,setError]=useState(''),[success,setSuccess]=useState('');
 const refresh=async()=>setBackups(await invoke<Backup[]>('list_backups'));
 useEffect(()=>{void refresh().catch(e=>setError(String(e)))},[]);
 const create=async()=>{setBusy(true);setError('');setSuccess('');try{const b=await invoke<Backup>('create_backup');setSuccess('تم إنشاء نسخة احتياطية وفحص سلامتها بنجاح: '+b.fileName);await refresh()}catch(e){setError(String(e))}finally{setBusy(false)}};
 return <div className="work-module">
 <h2>النسخ الاحتياطي المحلي</h2>
 <p className="hint">ينشئ البرنامج نسخة مستقلة من قاعدة البيانات باستخدام آلية SQLite للنسخ الحي، ثم ينفذ فحص integrity_check قبل اعتمادها. تحفظ النسخ داخل مجلد بيانات التطبيق على هذا الجهاز.</p>
 {error&&<p className="error-msg">{error}</p>}{success&&<p className="ok-msg">{success}</p>}
 <div className="notice"><b>حماية بياناتك</b><p>أنشئ نسخة احتياطية دورياً، وانسخ ملفات النسخ إلى وسيط خارجي آمن. وجود النسخة على الجهاز نفسه لا يحمي من تلف القرص أو فقدانه.</p><button className="primary" disabled={busy} onClick={()=>void create()}>{busy?'جارٍ إنشاء النسخة…':'إنشاء نسخة احتياطية الآن'}</button></div>
 <h3>النسخ المحفوظة</h3>
 <div className="table-wrap"><table><thead><tr><th>اسم الملف</th><th>تاريخ الإنشاء</th><th>الحجم</th><th>المسار المحلي</th></tr></thead><tbody>{backups.map(b=><tr key={b.fileName}><td>{b.fileName}</td><td>{dateLabel(b.createdUnix)}</td><td>{sizeLabel(b.sizeBytes)}</td><td dir="ltr">{b.path}</td></tr>)}</tbody></table>{!backups.length&&<p className="empty">لا توجد نسخ احتياطية بعد</p>}</div>
 </div>
}
