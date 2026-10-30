//! Bounded cover artwork, separate from executable and desktop shortcut icons.
use crate::{game_icons, windows_games};
use sha2::{Digest, Sha256};
use std::{fs::{self,File,OpenOptions}, io::{Read,Write}, os::unix::fs::{DirBuilderExt,OpenOptionsExt}, path::{Path,PathBuf}, sync::{Mutex,LazyLock}, time::{Duration,SystemTime,UNIX_EPOCH}};
const MAX_BYTES:usize=8*1024*1024;
static WRITES:Mutex<()>=Mutex::new(());
static MISSES:LazyLock<Mutex<std::collections::HashMap<String,std::time::Instant>>>=LazyLock::new(||Mutex::new(std::collections::HashMap::new()));
fn dimensions(bytes:&[u8])->Option<(&'static str,u32,u32)>{
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.get(12..16)==Some(b"IHDR") {
        if !bytes.ends_with(b"\x00\x00\x00\x00IEND\xaeB`\x82"){return None;}
        return Some(("image/png",u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?),u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?)));
    }
    if !bytes.starts_with(b"\xff\xd8") || !bytes.ends_with(b"\xff\xd9"){return None;}
    let mut at=2;let mut frame=None;let mut scan=false;
    while at<bytes.len(){
        if *bytes.get(at)?!=0xff{return None;}while bytes.get(at)==Some(&0xff){at+=1;}
        let marker=*bytes.get(at)?;at+=1;
        if marker==0xd9{return if scan && at==bytes.len(){frame}else{None};}
        if marker==0x01 || (0xd0..=0xd7).contains(&marker){continue;}
        let size=u16::from_be_bytes(bytes.get(at..at+2)?.try_into().ok()?) as usize;
        if size<2 || at.checked_add(size)?>bytes.len(){return None;}
        if matches!(marker,0xc0|0xc1|0xc2){
            if size<8{return None;}
            let height=u16::from_be_bytes(bytes.get(at+3..at+5)?.try_into().ok()?) as u32;
            let width=u16::from_be_bytes(bytes.get(at+5..at+7)?.try_into().ok()?) as u32;
            frame=Some(("image/jpeg",width,height));
        }
        at+=size;
        if marker==0xda{
            if frame.is_none(){return None;}scan=true;
            // Walk entropy-coded data, allowing stuffed bytes and restart markers.
            loop{
                let byte=*bytes.get(at)?;at+=1;if byte!=0xff{continue;}
                while bytes.get(at)==Some(&0xff){at+=1;}
                let next=*bytes.get(at)?;
                if next==0 || (0xd0..=0xd7).contains(&next){at+=1;continue;}
                at-=1;break;
            }
        }
    }None
}

fn checked(bytes:&[u8])->Result<String,String>{
    if bytes.len()>MAX_BYTES{return Err("Choose a cover smaller than 8 MiB".into());}
    let (mime,w,h)=dimensions(bytes).ok_or("Choose a complete PNG or JPEG cover")?;
    if !(300..=4096).contains(&w)||!(150..=4096).contains(&h)||u64::from(w)*u64::from(h)>16_000_000{return Err("Cover must be at least 300 × 150, at most 4096 per side, and under 16 megapixels".into());}
    if mime=="image/png" { ico::IconImage::read_png(std::io::Cursor::new(bytes)).map_err(|_|"The PNG cover could not be decoded")?; }
    Ok(game_icons::data_uri(bytes,mime))
}
fn read(path:&Path)->Result<Vec<u8>,String>{
    let mut f=File::options().read(true).custom_flags(libc::O_NOFOLLOW|libc::O_NONBLOCK).open(path).map_err(|e|e.to_string())?;
    let m=f.metadata().map_err(|e|e.to_string())?;if !m.is_file()||m.len()>MAX_BYTES as u64{return Err("Choose a regular cover file smaller than 8 MiB".into());}
    let mut bytes=Vec::new();Read::by_ref(&mut f).take(MAX_BYTES as u64+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;checked(&bytes)?;Ok(bytes)
}
fn directory(path:&Path)->Result<(),String>{
    let mut b=fs::DirBuilder::new();b.mode(0o700);match b.create(path){Ok(())=>{},Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>{},Err(e)=>return Err(e.to_string())}
    if !fs::symlink_metadata(path).map_err(|e|e.to_string())?.is_dir(){return Err("Cover storage must be a directory, not a symbolic link".into());}Ok(())
}
fn storage()->Result<PathBuf,String>{
    let base=std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p|p.is_absolute()).or_else(||std::env::var_os("HOME").map(|h|PathBuf::from(h).join(".local/share"))).ok_or("Application data directory unavailable")?;
    if !base.is_absolute(){return Err("Application data directory must be absolute".into());}
    fs::create_dir_all(&base).map_err(|e|e.to_string())?;let base=fs::canonicalize(base).map_err(|e|e.to_string())?;
    let app=base.join("proton-medic");directory(&app)?;let covers=app.join("covers");directory(&covers)?;Ok(covers)
}
fn identity(source:&str,id:&str,library:Option<&str>)->Result<(String,Vec<PathBuf>),String>{
    let roots=match source{"steam"=>game_icons::steam_roots(id,library.ok_or("Choose a scanned Steam library")?)?,"windows"=>{windows_games::validate_cover_id(id)?;Vec::new()},_=>return Err("Unknown game source".into())};
    let key=serde_json::to_vec(&(source,id,if source=="steam"{library}else{None})).map_err(|e|e.to_string())?;
    Ok((format!("{:x}",Sha256::digest(key)),roots))
}
fn stamp()->String{format!("{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos())}
fn save(dir:&Path,key:&str,bytes:&[u8],custom:bool)->Result<(),String>{
    let _guard=WRITES.lock().map_err(|_|"Cover storage busy")?;checked(bytes)?;
    let target=dir.join(format!("{key}-{}.image",if custom{"custom"}else{"steam"}));
    let _file_lock=crate::coordination::config(&target)?;
    if let Ok(meta)=fs::symlink_metadata(&target){
        if !meta.is_file(){return Err("Existing cover is not a regular file".into());}
        if custom {
            let old=read(&target)?;let backup=dir.join(format!("{key}-{}.backup",stamp()));
            let mut file=OpenOptions::new().write(true).create_new(true).mode(0o600).open(backup).map_err(|e|e.to_string())?;
            file.write_all(&old).and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;
        }
    }
    let temp=dir.join(format!(".{key}-{}.tmp",stamp()));
    let result=(||{let mut f=OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temp).map_err(|e|e.to_string())?;f.write_all(bytes).and_then(|_|f.sync_all()).map_err(|e|e.to_string())?;fs::rename(&temp,target).map_err(|e|e.to_string())?;File::open(dir).and_then(|f|f.sync_all()).map_err(|e|e.to_string())})();
    if result.is_err(){let _=fs::remove_file(temp);}result
}
fn cached(roots:&[PathBuf],id:&str)->Option<Vec<u8>>{
    for root in roots{
        let cache=root.join("appcache/librarycache");
        for name in [format!("{id}_library_hero.jpg"),format!("{id}_header.jpg"),format!("{id}_library_600x900.jpg"),format!("{id}_library_600x900_2x.jpg")]{if let Ok(bytes)=read(&cache.join(name)){return Some(bytes);}}
        let folder=cache.join(id);
        for name in ["library_hero.jpg","header.jpg","library_600x900.jpg","library_600x900_2x.jpg"]{if let Ok(bytes)=read(&folder.join(name)){return Some(bytes);}}
    }None
}
pub fn set(source:String,id:String,library_path:Option<String>,path:String,confirmed:bool)->Result<String,String>{
    if !confirmed{return Err("Approve choosing this cover first".into());}
    let (key,_)=identity(&source,&id,library_path.as_deref())?;let bytes=read(Path::new(&path))?;let uri=checked(&bytes)?;save(&storage()?,&key,&bytes,true)?;Ok(uri)
}
pub async fn get(source:String,id:String,library_path:Option<String>)->Result<Option<String>,String>{
    let steam=source=="steam";let game_id=id.clone();
    let (key,dir,local)=tauri::async_runtime::spawn_blocking(move||{
        let (key,roots)=identity(&source,&id,library_path.as_deref())?;let dir=storage()?;
        let custom=read(&dir.join(format!("{key}-custom.image"))).ok();
        let local=custom.or_else(||if steam{read(&dir.join(format!("{key}-steam.image"))).ok().or_else(||cached(&roots,&id))}else{None});
        Ok::<_,String>((key,dir,local))
    }).await.map_err(|e|e.to_string())??;
    if let Some(bytes)=local{return checked(&bytes).map(Some);}if !steam{return Ok(None);}
    if MISSES.lock().map_err(|_|"Cover cache busy")?.get(&key).is_some_and(|t|t.elapsed()<Duration::from_secs(300)){return Ok(None);}
    let _=rustls::crypto::ring::default_provider().install_default();
    let client=reqwest::Client::builder().https_only(true).redirect(reqwest::redirect::Policy::none()).connect_timeout(Duration::from_secs(4)).timeout(Duration::from_secs(8)).user_agent("Proton-Medic").build().map_err(|e|e.to_string())?;
    let url=format!("https://cdn.akamai.steamstatic.com/steam/apps/{game_id}/header.jpg");
    let download=async {
        let mut response=client.get(url).send().await.ok()?.error_for_status().ok()?;
        if response.content_length().is_some_and(|n|n>MAX_BYTES as u64){return None;}
        let mut bytes=Vec::new();while let Some(chunk)=response.chunk().await.ok()?{if bytes.len()+chunk.len()>MAX_BYTES{return None;}bytes.extend_from_slice(&chunk);}
        checked(&bytes).ok()?;Some(bytes)
    }.await;
    if let Some(bytes)=download{
        return tauri::async_runtime::spawn_blocking(move||{
            let _=save(&dir,&key,&bytes,false);
            // A selection made while the download was pending always wins.
            let bytes=read(&dir.join(format!("{key}-custom.image"))).unwrap_or(bytes);checked(&bytes).map(Some)
        }).await.map_err(|e|e.to_string())?;
    }
    if let Ok(mut misses)=MISSES.lock(){if misses.len()>=1024{misses.clear();}misses.insert(key,std::time::Instant::now());}Ok(None)
}
#[cfg(test)] mod tests{
    use super::*;
    struct Fixture(PathBuf);impl Fixture{fn new()->Self{let p=std::env::temp_dir().join(format!("pm-cover-{}",stamp()));fs::create_dir(&p).unwrap();Self(p)}}impl Drop for Fixture{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
    fn png(w:u32,h:u32)->Vec<u8>{let image=ico::IconImage::from_rgba_data(w,h,[30,100,180,255].repeat((w*h) as usize));let mut bytes=vec![];image.write_png(&mut bytes).unwrap();bytes}
    #[test]fn rejects_icons_truncation_and_oversized_dimensions(){assert!(checked(&png(32,32)).is_err());let mut bytes=png(400,200);assert!(checked(&bytes).is_ok());bytes.pop();assert!(checked(&bytes).is_err());assert!(checked(b"not an image").is_err());let mut bytes=png(400,200);bytes[16..20].copy_from_slice(&5000u32.to_be_bytes());assert!(checked(&bytes).is_err());}
    #[test]fn rejects_jpeg_without_a_complete_scan(){
        let header=[0xff,0xd8,0xff,0xc0,0,8,8,0,200,1,144,0,0xff,0xd9];
        assert!(checked(&header).is_err());
        let mut truncated=header.to_vec();truncated.truncate(9);assert!(checked(&truncated).is_err());
    }
    #[test]fn uses_header_artwork_without_selecting_icons(){let f=Fixture::new();let dir=f.0.join("appcache/librarycache");fs::create_dir_all(&dir).unwrap();fs::write(dir.join("123_icon.png"),png(400,200)).unwrap();assert!(cached(&[f.0.clone()],"123").is_none());fs::write(dir.join("123_header.jpg"),png(400,200)).unwrap();assert!(cached(&[f.0.clone()],"123").is_some());}
    #[test]fn copies_covers_and_backs_up_previous_selection(){let f=Fixture::new();let first=png(400,200);let second=png(500,250);save(&f.0,"safe",&first,true).unwrap();save(&f.0,"safe",&second,true).unwrap();assert_eq!(read(&f.0.join("safe-custom.image")).unwrap(),second);let backups:Vec<_>=fs::read_dir(&f.0).unwrap().filter_map(Result::ok).filter(|p|p.path().extension().is_some_and(|x|x=="backup")).collect();assert_eq!(backups.len(),1);assert_eq!(fs::read(backups[0].path()).unwrap(),first);}
    #[test]fn rejects_symlink_files_and_storage(){let f=Fixture::new();let file=f.0.join("cover.png");fs::write(&file,png(400,200)).unwrap();let link=f.0.join("link");std::os::unix::fs::symlink(&file,&link).unwrap();assert!(read(&link).is_err());assert!(directory(&link).is_err());std::os::unix::fs::symlink(&file,f.0.join("safe-custom.image")).unwrap();assert!(save(&f.0,"safe",&png(400,200),true).is_err());}
    #[test]fn concurrent_selection_cannot_skip_backups(){let f=Fixture::new();let target=f.0.join("safe-custom.image");let lock=crate::coordination::config(&target).unwrap();assert!(save(&f.0,"safe",&png(400,200),true).is_err());drop(lock);assert!(save(&f.0,"safe",&png(400,200),true).is_ok());}
    #[test]fn validates_source_before_storage(){assert!(set("other".into(),"../escape".into(),None,"/missing".into(),true).is_err());assert!(set("steam".into(),"1".into(),None,"/missing".into(),false).is_err());assert!(game_icons::steam_roots("../1","/Steam").is_err());}
}
