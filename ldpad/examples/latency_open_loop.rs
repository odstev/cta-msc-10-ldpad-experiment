//! Exploratory loopback AdmissionReview -> Rego -> SQLite WAL experiment.
//! Measures the scheduled client-to-response and server accept-to-commit paths.
//! The supplied LDPAD binary has no webhook/oplog, so this is a new test harness,
//! not a claim that this path already existed in the original implementation.
use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use hdrhistogram::Histogram;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use regorus::{Engine, Value as RegoValue};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, ffi::{c_char,c_void}, net::SocketAddr, path::PathBuf, time::{Duration,Instant}};
use tokio::{io::{AsyncReadExt,AsyncWriteExt}, net::{TcpListener,TcpStream}, sync::{mpsc,oneshot}, task::JoinHandle, time::{sleep_until,Instant as TokioInstant}};

const POLICY:&str = r#"
package ldpad.latency
import rego.v1
default allow := false
allow if {
    input.request.object.spec.securityContext.runAsNonRoot == true
}
"#;
const OBJECT_SIZES:[usize;5]=[1024,4096,10240,25600,51200];

#[derive(Parser,Debug)]
struct Args {
    #[arg(long)] db:PathBuf,
    #[arg(long)] histogram_output:PathBuf,
    #[arg(long,default_value_t=30)] seeds:u32,
    #[arg(long,default_value_t=10)] requests_per_seed:usize,
    #[arg(long,default_value="1,10,100")] rates:String,
}

#[link(name="dl")]
unsafe extern "C" { fn dlsym(handle:*mut c_void, symbol:*const c_char) -> *mut c_void; }
type ReadProbe = unsafe extern "C" fn() -> u64;
struct Probe { ns:Option<ReadProbe>, count:Option<ReadProbe> }
impl Probe {
    fn load() -> Self {
        unsafe {
            let n=dlsym(std::ptr::null_mut(),c"ldp_sync_ns".as_ptr());
            let c=dlsym(std::ptr::null_mut(),c"ldp_sync_calls".as_ptr());
            Self { ns:if n.is_null(){None}else{Some(std::mem::transmute::<*mut c_void,ReadProbe>(n))},
                   count:if c.is_null(){None}else{Some(std::mem::transmute::<*mut c_void,ReadProbe>(c))} }
        }
    }
    fn active(&self)->bool { self.ns.is_some() && self.count.is_some() }
    fn read(&self)->(u64,u64) { unsafe { (self.ns.map_or(0,|f|f()),self.count.map_or(0,|f|f())) } }
}
struct Job { body:Vec<u8>, accepted:Instant, reply:oneshot::Sender<std::result::Result<Stats,String>> }
struct Stats { uid:String, allowed:bool, service_ns:u64, commit_ns:u64, sync_ns:u64, sync_calls:u64, probe_active:bool }
#[derive(Clone)]
struct Record { seed:u32, rate:u32, order:usize, seq:usize, object_bytes:usize, request_bytes:usize,
                scheduler_lag_ns:u64, client_ns:u64, service_ns:u64, commit_ns:u64, sync_ns:u64,
                sync_calls:u64, probe_active:bool, allowed:bool }
fn ns(d:Duration)->u64 { d.as_nanos().min(u64::MAX as u128) as u64 }
fn elapsed_since(later:Instant,earlier:Instant)->u64 { ns(later.saturating_duration_since(earlier)) }

fn fixture(seed:u32,rate:u32,seq:usize,target:usize)->Result<(Vec<u8>,bool)> {
    let allowed=(seq+seed as usize)%5 != 0;
    let uid=format!("seed-{seed}-rate-{rate}-seq-{seq}");
    let mut object=json!({"apiVersion":"v1","kind":"Pod","metadata":{"name":"synthetic-registry","annotations":{"padding":""}},"spec":{"securityContext":{"runAsNonRoot":allowed},"containers":[{"name":"registry","image":"example.invalid/registry:1"}]}});
    let baseline=serde_json::to_vec(&object)?.len();
    if baseline>target { bail!("target object size too small"); }
    object["metadata"]["annotations"]["padding"]=Value::String("x".repeat(target-baseline));
    if serde_json::to_vec(&object)?.len()!=target { bail!("object size mismatch"); }
    let review=json!({"apiVersion":"admission.k8s.io/v1","kind":"AdmissionReview","request":{"uid":uid,"kind":{"group":"","version":"v1","kind":"Pod"},"object":object}});
    Ok((serde_json::to_vec(&review)?,allowed))
}

fn serve_worker(mut rx:mpsc::UnboundedReceiver<Job>,db:PathBuf,ready:oneshot::Sender<()>)->Result<()> {
    let mut conn=Connection::open(&db).context("open SQLite database")?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA wal_autocheckpoint=1000; CREATE TABLE IF NOT EXISTS oplog (uid TEXT PRIMARY KEY, allowed INTEGER NOT NULL, object_sha256 TEXT NOT NULL, request_bytes INTEGER NOT NULL);")?;
    let sync:String=conn.query_row("PRAGMA journal_mode",[],|row|row.get(0))?;
    let mode:i64=conn.query_row("PRAGMA synchronous",[],|row|row.get(0))?;
    if sync.to_lowercase()!="wal" || mode!=2 {bail!("SQLite must use WAL and synchronous FULL");}
    let mut engine=Engine::new();
    engine.add_policy("latency_policy.rego".to_string(),POLICY.to_string())?;
    let probe=Probe::load();
    eprintln!("worker ready: sqlite journal={sync}, synchronous=FULL, sync_probe={}",probe.active());
    let _=ready.send(());
    while let Some(job)=rx.blocking_recv() {
        let result=(||->Result<Stats>{
            let parsed:Value=serde_json::from_slice(&job.body)?;
            if parsed["apiVersion"]!="admission.k8s.io/v1" || parsed["kind"]!="AdmissionReview" {bail!("invalid AdmissionReview envelope");}
            let uid=parsed["request"]["uid"].as_str().ok_or_else(||anyhow!("missing uid"))?.to_string();
            let object=serde_json::to_vec(&parsed["request"]["object"])?;
            let digest=format!("{:x}",Sha256::digest(&object));
            engine.set_input(RegoValue::from_json_str(std::str::from_utf8(&job.body)?)?);
            let decision=engine.eval_rule("data.ldpad.latency.allow".to_string())?;
            let allowed=decision==RegoValue::from(true);
            let tx=conn.transaction()?;
            tx.execute("INSERT INTO oplog(uid, allowed, object_sha256, request_bytes) VALUES (?1, ?2, ?3, ?4)",params![uid, i64::from(allowed),digest,job.body.len() as i64])?;
            let (sync_start,calls_start)=probe.read();
            let commit_start=Instant::now();
            tx.commit().context("commit SQLite transaction")?;
            let commit_ns=ns(commit_start.elapsed());
            let (sync_end,calls_end)=probe.read();
            let service_ns=ns(job.accepted.elapsed());
            Ok(Stats{uid,allowed,service_ns,commit_ns,sync_ns:sync_end.saturating_sub(sync_start),
                     sync_calls:calls_end.saturating_sub(calls_start),probe_active:probe.active()})
        })().map_err(|e|format!("{e:#}"));
        let _=job.reply.send(result);
    }
    Ok(())
}

fn header_end(data:&[u8])->Option<usize> { data.windows(4).position(|w|w==b"\r\n\r\n").map(|n|n+4) }
async fn handle(mut stream:TcpStream, accepted:Instant,tx:mpsc::UnboundedSender<Job>)->Result<()> {
    let mut data=Vec::with_capacity(65536); let mut buffer=[0_u8;8192];
    let end=loop {
        if let Some(end)=header_end(&data){break end;}
        let n=stream.read(&mut buffer).await?;
        if n==0 {bail!("connection closed before HTTP headers");}
        data.extend_from_slice(&buffer[..n]);
        if data.len()>131072 {bail!("HTTP request too large");}
    };
    let headers=std::str::from_utf8(&data[..end])?;
    let content_length=headers.lines().find_map(|line|{
        let (key,value)=line.split_once(':')?;
        if key.eq_ignore_ascii_case("content-length"){value.trim().parse::<usize>().ok()}else{None}
    }).ok_or_else(||anyhow!("missing content length"))?;
    if content_length>100000 {bail!("request body too large");}
    while data.len()-end<content_length {
        let n=stream.read(&mut buffer).await?;
        if n==0 {bail!("connection closed before HTTP body");}
        data.extend_from_slice(&buffer[..n]);
    }
    let body=data[end..end+content_length].to_vec();
    let (reply,receiver)=oneshot::channel();
    tx.send(Job{body,accepted,reply}).map_err(|_|anyhow!("worker unavailable"))?;
    let stats=receiver.await.map_err(|_|anyhow!("worker response lost"))?.map_err(|e|anyhow!(e))?;
    let response=serde_json::to_vec(&json!({"apiVersion":"admission.k8s.io/v1","kind":"AdmissionReview","response":{"uid":stats.uid,"allowed":stats.allowed}}))?;
    let header=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nX-LDP-Service-Ns: {}\r\nX-LDP-Commit-Ns: {}\r\nX-LDP-Sync-Ns: {}\r\nX-LDP-Sync-Calls: {}\r\nX-LDP-Probe: {}\r\n\r\n",response.len(),stats.service_ns,stats.commit_ns,stats.sync_ns,stats.sync_calls,u8::from(stats.probe_active));
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(&response).await?;
    stream.shutdown().await?;
    Ok(())
}

async fn client(addr:SocketAddr,scheduled:Instant,seed:u32,rate:u32,order:usize,seq:usize,target:usize,body:Vec<u8>,expected:bool)->Result<Record>{
    let started=Instant::now();
    let mut stream=TcpStream::connect(addr).await?;
    let request=format!("POST /admit HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
    stream.write_all(request.as_bytes()).await?;
    stream.write_all(&body).await?;
    let mut response=Vec::new();
    stream.read_to_end(&mut response).await?;
    let done=Instant::now();
    let end=header_end(&response).ok_or_else(||anyhow!("incomplete response"))?;
    let head=std::str::from_utf8(&response[..end])?;
    if !head.starts_with("HTTP/1.1 200 "){bail!("HTTP failure");}
    let mut hdrs=BTreeMap::new();
    for line in head.lines().skip(1) { if let Some((k,v))=line.split_once(':'){hdrs.insert(k.to_ascii_lowercase(),v.trim().to_string());} }
    let read_hdr=|key:&str|->Result<u64>{Ok(hdrs.get(key).ok_or_else(||anyhow!("missing {key}"))?.parse()?)};
    let response_body:Value=serde_json::from_slice(&response[end..])?;
    let allowed=response_body["response"]["allowed"].as_bool().ok_or_else(||anyhow!("missing decision"))?;
    let expected_uid=format!("seed-{seed}-rate-{rate}-seq-{seq}");
    if allowed!=expected || response_body["response"]["uid"].as_str()!=Some(&expected_uid){bail!("policy decision or UID mismatch");}
    Ok(Record{seed,rate,order,seq,object_bytes:target,request_bytes:body.len(),
        scheduler_lag_ns:elapsed_since(started,scheduled),client_ns:elapsed_since(done,scheduled),
        service_ns:read_hdr("x-ldp-service-ns")?,commit_ns:read_hdr("x-ldp-commit-ns")?,
        sync_ns:read_hdr("x-ldp-sync-ns")?,sync_calls:read_hdr("x-ldp-sync-calls")?,
        probe_active:read_hdr("x-ldp-probe")?==1,allowed})
}

fn hist_report(path:PathBuf,records:&[Record],rates:&[u32])->Result<()> {
    let mut f=std::fs::File::create(path)?;
    use std::io::Write;
    writeln!(f,"rate_hz,method,count,p50_ns,p95_ns,p99_ns,p999_ns,max_ns,expected_interval_ns")?;
    for &rate in rates {
        let mut raw=Histogram::<u64>::new_with_bounds(1,60_000_000_000,3)?;
        let mut corrected=Histogram::<u64>::new_with_bounds(1,60_000_000_000,3)?;
        let interval=1_000_000_000_u64/u64::from(rate);
        for r in records.iter().filter(|r|r.rate==rate){raw.record(r.client_ns.max(1))?;corrected.record_correct(r.client_ns.max(1),interval)?;}
        for (name,h) in [("open_loop_raw",raw),("record_correct_sensitivity",corrected)]{
            writeln!(f,"{rate},{name},{},{},{},{},{},{},{}",h.len(),h.value_at_quantile(0.50),h.value_at_quantile(0.95),h.value_at_quantile(0.99),h.value_at_quantile(0.999),h.max(),interval)?;
        }
    }
    Ok(())
}

#[tokio::main(flavor="multi_thread",worker_threads=4)]
async fn main()->Result<()> {
    let args=Args::parse();
    if args.seeds==0 || args.requests_per_seed==0 {bail!("seeds and requests must be positive");}
    if args.db.exists(){bail!("database already exists; provide a fresh path");}
    let rates:Vec<u32>=args.rates.split(',').map(|x|x.parse::<u32>()).collect::<std::result::Result<_,_>>()?;
    if rates.is_empty() || rates.iter().any(|&v|v==0 || v>1000){bail!("rates must be between 1 and 1000");}
    let listener=TcpListener::bind("127.0.0.1:0").await?;let addr=listener.local_addr()?;
    let (tx,rx)=mpsc::unbounded_channel();
    let (ready_tx,ready_rx)=oneshot::channel();
    let worker=std::thread::spawn(move||serve_worker(rx,args.db,ready_tx));
    ready_rx.await.context("worker did not become ready")?;
    let server_tx=tx.clone();
    let server=tokio::spawn(async move {loop{match listener.accept().await {
        Ok((stream,_))=>{let accepted=Instant::now();let sender=server_tx.clone();tokio::spawn(async move{if let Err(e)=handle(stream,accepted,sender).await{eprintln!("server error: {e:#}");}});},
        Err(e)=>{eprintln!("listener error: {e}");break;}
    }}});
    let mut all=Vec::new();
    for seed in 0..args.seeds {
        let mut rng=StdRng::seed_from_u64(u64::from(seed)+10101);
        let mut run_rates=rates.clone();run_rates.shuffle(&mut rng);
        let mut sizes:Vec<usize>=(0..args.requests_per_seed).map(|i|OBJECT_SIZES[i%OBJECT_SIZES.len()]).collect();sizes.shuffle(&mut rng);
        for (order,rate) in run_rates.into_iter().enumerate(){
            let mut fixtures=Vec::new();
            for (seq,&size) in sizes.iter().enumerate(){let (body,expected)=fixture(seed,rate,seq,size)?;fixtures.push((seq,size,body,expected));}
            let first=TokioInstant::now()+Duration::from_millis(50);
            let interval_ns=1_000_000_000_u64/u64::from(rate);
            let mut tasks:Vec<JoinHandle<Result<Record>>>=Vec::new();
            for (seq,size,body,expected) in fixtures {
                let scheduled=first+Duration::from_nanos(interval_ns*seq as u64);
                sleep_until(scheduled).await;
                tasks.push(tokio::spawn(client(addr,scheduled.into_std(),seed,rate,order,seq,size,body,expected)));
            }
            for task in tasks {all.push(task.await??);}
        }
        eprintln!("completed seed {} of {}",seed+1,args.seeds);
    }
    all.sort_by_key(|r|(r.seed,r.rate,r.seq));
    hist_report(args.histogram_output,&all,&rates)?;
    println!("seed,rate_hz,condition_order,sequence,object_bytes,request_bytes,scheduler_lag_ns,scheduled_to_response_ns,accept_to_commit_ns,commit_ns,sync_ns,sync_calls,probe_active,allowed");
    for r in &all {println!("{},{},{},{},{},{},{},{},{},{},{},{},{},{}",r.seed,r.rate,r.order,r.seq,r.object_bytes,r.request_bytes,r.scheduler_lag_ns,r.client_ns,r.service_ns,r.commit_ns,r.sync_ns,r.sync_calls,u8::from(r.probe_active),u8::from(r.allowed));}
    server.abort();drop(tx);
    tokio::task::spawn_blocking(move||worker.join().map_err(|_|anyhow!("worker panicked"))?).await??;
    Ok(())
}
