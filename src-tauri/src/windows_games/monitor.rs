//! Opt-in, expiring resource samples shared with desktop shortcut launchers.
use super::*;
use crate::runtime_session::Process;
use std::time::{Duration, Instant};
#[derive(Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
struct Request { expires_at: u64 }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Sample { pub run_id:String, pub captured_at:u64, pub cpu_percent:Option<f64>, pub memory_bytes:u64, pub peak_memory_bytes:u64, pub process_count:usize, pub partial:bool }
#[derive(Serialize)]
pub struct Report { pub status:String, pub sample:Option<Sample>, pub message:String }
fn now()->u64 {SystemTime::now().duration_since(UNIX_EPOCH).map(|d|d.as_millis() as u64).unwrap_or(0)}
fn read<T:serde::de::DeserializeOwned>(dir:&Path,name:&str)->Result<Option<T>,String>{
    directory(dir,false)?;let path=dir.join(name);
    if fs::symlink_metadata(&path).is_err_and(|e|e.kind()==std::io::ErrorKind::NotFound){return Ok(None);}
    regular(&path,16384)?;serde_json::from_slice(&fs::read(path).map_err(|e|e.to_string())?).map(Some).map_err(|e|e.to_string())
}
fn write<T:Serialize>(dir:&Path,name:&str,value:&T)->Result<(),String>{
    directory(dir,false)?;let target=dir.join(name);if target.exists(){regular(&target,16384)?;}
    let temp=dir.join(format!(".monitor-{}.tmp",stamp()?));
    let result=(||{let mut file=OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temp).map_err(|e|e.to_string())?;
        file.write_all(&serde_json::to_vec(value).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        fs::rename(&temp,&target).map_err(|e|e.to_string())})();
    if result.is_err(){let _=fs::remove_file(temp);}result
}
fn requested(expires:u64,at:u64)->bool {expires>at && expires.saturating_sub(at)<=8000}
fn fresh(sample:&Sample,run:&str,at:u64)->bool {
    sample.run_id==run && sample.captured_at<=at && at-sample.captured_at<=4000 && sample.cpu_percent.is_none_or(|n|n.is_finite()&&n>=0.0)
}
fn run_directory(dir:&Path,run:&str)->Result<PathBuf,String>{
    if !valid_id(run){return Err("Invalid run identity".into());}
    let root=dir.join("logs");directory(&root,false)?;let logs=root.join(run);directory(&logs,false)?;Ok(logs)
}
pub fn sample(id:String,run_id:String)->Result<Report,String>{
    let(dir,record)=load(&root()?,&id)?;
    if record.run_id.as_deref()!=Some(run_id.as_str()) || !record.state.ends_with("running") || record.boot_id.is_none() || record.boot_id!=boot_id(){
        return Ok(Report{status:"inactive".into(),sample:None,message:"This run is no longer available for monitoring.".into()});
    }
    let logs=run_directory(&dir,&run_id)?;let at=now();
    write(&logs,"monitor-request.json",&Request{expires_at:at.saturating_add(6000)})?;
    let current:Option<Sample>=read(&logs,"monitor-sample.json")?;
    match current {
        Some(sample) if fresh(&sample,&run_id,at)=>Ok(Report{status:"live".into(),sample:Some(sample),message:String::new()}),
        Some(_)=>Ok(Report{status:"stale".into(),sample:None,message:"No fresh process sample is available. Waiting for the launcher.".into()}),
        None=>Ok(Report{status:"waiting".into(),sample:None,message:"Waiting for a process sample…".into()}),
    }
}
pub fn stop(id:String,run_id:String)->Result<(),String>{
    let(dir,_)=load(&root()?,&id)?;let logs=run_directory(&dir,&run_id)?;
    write(&logs,"monitor-request.json",&Request{expires_at:0})
}
pub(crate) struct Sampler {
    logs:PathBuf, run_id:String, checked:Option<Instant>, previous:Option<(Instant,HashMap<(u32,u64),u64>)>, peak:u64,
    ticks_per_second:f64, page_size:u64,
}
impl Sampler {
    pub(crate) fn new(logs:&Path)->Self {
        let ticks=unsafe{libc::sysconf(libc::_SC_CLK_TCK)};let page=unsafe{libc::sysconf(libc::_SC_PAGESIZE)};
        Self{logs:logs.into(),run_id:logs.file_name().map(|s|s.to_string_lossy().into_owned()).unwrap_or_default(),checked:None,previous:None,peak:0,ticks_per_second:ticks.max(0) as f64,page_size:page.max(0) as u64}
    }
    pub(crate) fn observe(&mut self,all:&[Process],known:&HashMap<u32,u64>){
        let instant=Instant::now();if self.checked.is_some_and(|last|instant.duration_since(last)<Duration::from_secs(1)){return;}
        self.checked=Some(instant);let at=now();
        let active=read::<Request>(&self.logs,"monitor-request.json").ok().flatten().is_some_and(|r|requested(r.expires_at,at));
        if !active {self.previous=None;self.peak=0;return;}
        if self.ticks_per_second<=0.0 || self.page_size==0 {return;}
        let sample=self.calculate(all,known,instant,at);
        // Metrics must never interrupt the game, output capture, or completion receipt.
        let _=write(&self.logs,"monitor-sample.json",&sample);
    }
    fn calculate(&mut self,all:&[Process],known:&HashMap<u32,u64>,instant:Instant,at:u64)->Sample {
        let mut counters=HashMap::new();let mut memory=0u64;let mut count=0;let mut partial=false;
        for process in all.iter().filter(|p|known.get(&p.pid)==Some(&p.start)&&!matches!(p.state,'Z'|'X')) {
            count+=1;
            if let Some((ticks,pages))=process.metrics {counters.insert((process.pid,process.start),ticks);memory=memory.saturating_add(pages.saturating_mul(self.page_size));}else{partial=true;}
        }
        let cpu=self.previous.as_ref().and_then(|(previous,old)|{
            let elapsed=instant.duration_since(*previous).as_secs_f64();if elapsed<=0.0{return None;}
            let ticks=counters.iter().filter_map(|(key,value)|old.get(key).map(|before|value.saturating_sub(*before))).fold(0u64,u64::saturating_add);
            Some(ticks as f64/self.ticks_per_second/elapsed*100.0)
        });
        self.peak=self.peak.max(memory);self.previous=Some((instant,counters));
        Sample{run_id:self.run_id.clone(),captured_at:at,cpu_percent:cpu,memory_bytes:memory,peak_memory_bytes:self.peak,process_count:count,partial}
    }
}

#[cfg(test)] mod tests {
    use super::*;
    fn process(pid:u32,start:u64,ticks:u64,pages:u64)->Process{Process{pid,parent:1,group:1,start,state:'S',metrics:Some((ticks,pages))}}
    fn sampler()->Sampler{let mut s=Sampler::new(Path::new("/unused/1-100"));s.ticks_per_second=100.0;s.page_size=4096;s}
    #[test] fn cpu_uses_deltas_for_matching_process_identities_and_can_exceed_one_core(){
        let mut s=sampler();let at=Instant::now();let known=HashMap::from([(1,10),(2,20)]);
        let first=s.calculate(&[process(1,10,500,10),process(2,20,200,5)],&known,at,1000);assert_eq!(first.cpu_percent,None);
        let next=s.calculate(&[process(1,10,700,12),process(2,20,300,7)],&known,at+Duration::from_secs(2),3000);
        assert_eq!(next.cpu_percent,Some(150.0));assert_eq!(next.memory_bytes,19*4096);assert_eq!(next.process_count,2);
    }
    #[test] fn reused_pids_new_children_and_departed_processes_do_not_create_cpu_spikes(){
        let mut s=sampler();let at=Instant::now();s.calculate(&[process(1,10,500,10)],&HashMap::from([(1,10)]),at,1000);
        let next=s.calculate(&[process(1,99,900000,5),process(2,20,800000,3)],&HashMap::from([(1,99),(2,20)]),at+Duration::from_secs(1),2000);
        assert_eq!(next.cpu_percent,Some(0.0));assert_eq!(next.memory_bytes,8*4096);assert_eq!(next.peak_memory_bytes,10*4096);
    }
    #[test] fn totals_exclude_unrelated_reused_and_zombie_processes_and_mark_partial_reads(){
        let mut s=sampler();let mut zombie=process(2,20,50,1000);zombie.state='Z';let mut missing=process(3,30,10,10);missing.metrics=None;
        let sample=s.calculate(&[process(1,10,100,5),zombie,missing,process(4,40,100,1000),process(5,99,100,1000)],&HashMap::from([(1,10),(2,20),(3,30),(5,50)]),Instant::now(),1000);
        assert_eq!(sample.memory_bytes,5*4096);assert_eq!(sample.process_count,2);assert!(sample.partial);
    }
    #[test] fn requests_expire_and_cannot_enable_monitoring_indefinitely(){
        assert!(requested(7000,1000));assert!(!requested(1000,1000));assert!(!requested(999,1000));assert!(!requested(u64::MAX,1000));
    }
    #[test] fn old_future_or_wrong_run_samples_are_never_live(){
        let mut s=sampler();let mut sample=s.calculate(&[],&HashMap::new(),Instant::now(),1000);
        assert!(fresh(&sample,"1-100",2000));assert!(!fresh(&sample,"2-100",2000));assert!(!fresh(&sample,"1-100",999));assert!(!fresh(&sample,"1-100",5001));
        sample.cpu_percent=Some(f64::NAN);assert!(!fresh(&sample,"1-100",2000));
    }
    #[test] fn disabled_monitor_writes_no_samples_and_stop_clears_the_baseline(){
        let dir=std::env::temp_dir().join(format!("pm-monitor-{}",stamp().unwrap()));fs::create_dir(&dir).unwrap();let mut s=Sampler::new(&dir);
        s.observe(&[],&HashMap::new());assert!(!dir.join("monitor-sample.json").exists());
        write(&dir,"monitor-request.json",&Request{expires_at:now()+6000}).unwrap();s.checked=None;s.observe(&[],&HashMap::new());assert!(dir.join("monitor-sample.json").is_file());assert!(s.previous.is_some());
        let before=fs::read(dir.join("monitor-sample.json")).unwrap();write(&dir,"monitor-request.json",&Request{expires_at:0}).unwrap();s.checked=None;s.observe(&[],&HashMap::new());
        assert!(s.previous.is_none());assert_eq!(s.peak,0);assert_eq!(fs::read(dir.join("monitor-sample.json")).unwrap(),before);fs::remove_dir_all(dir).unwrap();
    }
    #[test] fn sample_io_rejects_symlinks_and_oversized_files(){
        let dir=std::env::temp_dir().join(format!("pm-monitor-{}",stamp().unwrap()));fs::create_dir(&dir).unwrap();let file=dir.join("monitor-request.json");
        fs::write(&file,vec![b'x';16385]).unwrap();assert!(read::<Request>(&dir,"monitor-request.json").is_err());fs::remove_file(&file).unwrap();
        std::os::unix::fs::symlink("/etc/passwd",&file).unwrap();assert!(read::<Request>(&dir,"monitor-request.json").is_err());assert!(write(&dir,"monitor-request.json",&Request{expires_at:0}).is_err());fs::remove_dir_all(dir).unwrap();
    }
}
