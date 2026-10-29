//! Official GE-Proton releases installed side by side, without executing archive content.
use crate::{inventory, scanner};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::{collections::HashSet, fs::{self, File, OpenOptions}, io::{Read, Write}, os::unix::fs::{OpenOptionsExt, PermissionsExt}, path::{Component, Path, PathBuf}, sync::{Mutex, atomic::{AtomicBool, Ordering}}, time::{Duration, SystemTime, UNIX_EPOCH}};
const API: &str = "https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases";
const ASSETS: &str = "https://github.com/GloriousEggroll/proton-ge-custom/releases/download/";
const MAX_DOWNLOAD: u64 = 2 * 1024 * 1024 * 1024;
const MAX_EXPANDED: u64 = 8 * 1024 * 1024 * 1024;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Release { pub tag:String, pub name:String, pub size:u64, url:String, checksum_url:String }
#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Status { pub phase:String, pub tag:String, pub downloaded:u64, pub total:u64, pub message:String }
static STATUS: Mutex<Option<Status>> = Mutex::new(None);
static BUSY: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Local { pub installations:Vec<scanner::SteamInstallation>, pub tools:Vec<scanner::CompatibilityTool>, pub status:Option<Status>, pub umu_available:bool }
pub fn local() -> Local {
    let scan=inventory::steam();
    Local { installations:scan.installations.clone(),tools:scan.compatibility_tools.clone(),status:status(),umu_available:umu().is_some() }
}
pub fn status()->Option<Status>{ STATUS.lock().ok().and_then(|s|s.clone()) }
fn update(phase:&str,tag:&str,downloaded:u64,total:u64,message:&str){if let Ok(mut s)=STATUS.lock(){*s=Some(Status{phase:phase.into(),tag:tag.into(),downloaded,total,message:message.into()});}}
pub fn cancel()->Result<(),String>{if BUSY.load(Ordering::Acquire){CANCEL.store(true,Ordering::Release);}Ok(())}
fn canceled()->Result<(),String>{if CANCEL.load(Ordering::Acquire){Err("Download canceled. Installed versions are unchanged.".into())}else{Ok(())}}
fn safe_name(s:&str)->bool{!s.is_empty()&&s.len()<150&&s.bytes().all(|b|b.is_ascii_alphanumeric()||matches!(b,b'-'|b'_'|b'.'))&&!s.starts_with('.')}
fn client()->Result<reqwest::Client,String>{
    reqwest::Client::builder().user_agent("Proton-Medic").https_only(true).connect_timeout(Duration::from_secs(20)).read_timeout(Duration::from_secs(30)).timeout(Duration::from_secs(3600))
        .redirect(reqwest::redirect::Policy::custom(|attempt|{
            let allowed=matches!(attempt.url().host_str(),Some("github.com"|"api.github.com"|"release-assets.githubusercontent.com"|"objects.githubusercontent.com"));
            if attempt.previous().len()>5 || attempt.url().scheme()!="https" || !allowed {attempt.error("Untrusted release redirect")}else{attempt.follow()}
        })).build().map_err(|e|e.to_string())
}
async fn small(url:&str,limit:usize)->Result<Vec<u8>,String>{
    let mut response=client()?.get(url).timeout(Duration::from_secs(45)).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|format!("GitHub request failed (check connection or rate limit): {e}"))?;
    let mut bytes=Vec::new();while let Some(chunk)=response.chunk().await.map_err(|e|e.to_string())?{if bytes.len()+chunk.len()>limit{return Err("Release metadata exceeds the size limit".into());}bytes.extend_from_slice(&chunk);}Ok(bytes)
}
#[derive(Deserialize)] struct ApiRelease { tag_name:String, draft:bool, prerelease:bool, assets:Vec<ApiAsset> }
#[derive(Deserialize)] struct ApiAsset { name:String, size:u64, browser_download_url:String }
fn release_from(raw:ApiRelease)->Option<Release>{
    if raw.draft||raw.prerelease||!raw.tag_name.starts_with("GE-Proton")||!safe_name(&raw.tag_name){return None;}
    let current=format!("{}-x86_64.tar.gz",raw.tag_name);let legacy=format!("{}.tar.gz",raw.tag_name);
    let asset=raw.assets.iter().find(|a|a.name==current).or_else(||raw.assets.iter().find(|a|a.name==legacy))?;
    if asset.size==0||asset.size>MAX_DOWNLOAD{return None;}
    let checksum_name=format!("{}.sha512sum",asset.name.strip_suffix(".tar.gz")?);
    let checksum=raw.assets.iter().find(|a|a.name==checksum_name&&a.size<=4096)?;
    let base=format!("{ASSETS}{}/",raw.tag_name);
    if asset.browser_download_url!=format!("{base}{}",asset.name)||checksum.browser_download_url!=format!("{base}{checksum_name}"){return None;}
    Some(Release{tag:raw.tag_name,name:asset.name.clone(),size:asset.size,url:asset.browser_download_url.clone(),checksum_url:checksum.browser_download_url.clone()})
}
pub async fn catalog(page:u32)->Result<Vec<Release>,String>{
    if !(1..=10).contains(&page){return Err("Invalid release page".into());}
    let bytes=small(&format!("{API}?per_page=30&page={page}"),4*1024*1024).await?;
    let releases:Vec<ApiRelease>=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
    Ok(releases.into_iter().filter_map(release_from).collect())
}
fn target(root:&str)->Result<PathBuf,String>{
    let scan=scanner::scan();let install=scan.installations.iter().find(|i|i.path==root).ok_or("Steam installation is no longer available. Refresh the list.")?;
    let base=fs::canonicalize(&install.path).map_err(|e|e.to_string())?;
    let path=base.join("compatibilitytools.d");
    match fs::symlink_metadata(&path){Ok(m) if !m.is_dir()||m.file_type().is_symlink()=>return Err("Compatibility tools folder must be a real directory".into()),Ok(_)=>{},Err(e) if e.kind()==std::io::ErrorKind::NotFound=>fs::create_dir(&path).map_err(|e|e.to_string())?,Err(e)=>return Err(e.to_string())}Ok(path)
}
fn expected_hash(text:&[u8],name:&str)->Result<String,String>{
    let text=std::str::from_utf8(text).map_err(|_|"Invalid checksum text")?;let fields=text.split_whitespace().collect::<Vec<_>>();
    if fields.len()!=2||fields[0].len()!=128||!fields[0].bytes().all(|b|b.is_ascii_hexdigit())||fields[1].trim_start_matches('*')!=name{return Err("Checksum file does not match the selected archive".into());}Ok(fields[0].to_ascii_lowercase())
}
fn relative(path:&Path)->Result<PathBuf,String>{
    let mut out=PathBuf::new();for part in path.components(){match part{Component::Normal(s)=>out.push(s),Component::CurDir=>{},_=>return Err("Archive contains an unsafe path".into())}}
    if out.as_os_str().is_empty(){return Err("Archive contains an empty path".into());}Ok(out)
}
fn link_target(path:&Path,link:&Path,hard:bool,root:&Path)->Result<PathBuf,String>{
    let mut out=if hard{PathBuf::new()}else{path.parent().ok_or("Invalid link parent")?.to_path_buf()};
    for c in link.components(){match c{Component::Normal(s)=>out.push(s),Component::CurDir=>{},Component::ParentDir=>{if !out.pop(){return Err("Archive link escapes its folder".into());}},_=>return Err("Archive contains an absolute link".into())}}
    if !out.starts_with(root){return Err("Archive link escapes its Proton folder".into());}Ok(out)
}
fn real_parents(stage:&Path,path:&Path)->Result<(),String>{
    let mut cursor=stage.to_path_buf();for c in path.parent().unwrap_or(Path::new("")).components(){cursor.push(c);match fs::symlink_metadata(&cursor){Ok(m) if m.is_dir()&&!m.file_type().is_symlink()=>{},Ok(_)=>return Err("Archive parent is not a directory".into()),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>fs::create_dir(&cursor).map_err(|e|e.to_string())?,Err(e)=>return Err(e.to_string())}}Ok(())
}
fn extract(reader:impl Read,stage:&Path)->Result<PathBuf,String>{
    let mut archive=tar::Archive::new(reader);let mut root:Option<PathBuf>=None;let mut seen=HashSet::new();let mut links=vec![];let mut total=0u64;
    for (index,item) in archive.entries().map_err(|e|e.to_string())?.enumerate(){
        canceled()?;if index>=200_000{return Err("Archive has too many files".into());}
        let mut entry=item.map_err(|e|e.to_string())?;let path=relative(&entry.path().map_err(|e|e.to_string())?)?;
        let first=PathBuf::from(path.components().next().ok_or("Empty archive path")?.as_os_str());
        if root.as_ref().is_some_and(|r|r!=&first){return Err("Archive must contain one Proton folder".into());}root=Some(first.clone());
        let ty=entry.header().entry_type();if !seen.insert(path.clone())&&!ty.is_dir(){return Err("Archive contains duplicate files".into());}
        real_parents(stage,&path)?;let destination=stage.join(&path);
        if ty.is_dir(){if destination.exists(){if !destination.is_dir(){return Err("Archive directory conflict".into());}}else{fs::create_dir(&destination).map_err(|e|e.to_string())?;}continue;}
        if ty.is_symlink()||ty.is_hard_link(){let link=entry.link_name().map_err(|e|e.to_string())?.ok_or("Missing link target")?.into_owned();let resolved=link_target(&path,&link,ty.is_hard_link(),&first)?;links.push((path,link,resolved,ty.is_hard_link()));continue;}
        if !ty.is_file(){return Err("Archive contains unsupported special files".into());}
        let size=entry.size();total=total.checked_add(size).ok_or("Archive size overflow")?;if total>MAX_EXPANDED{return Err("Expanded Proton exceeds 8 GiB".into());}
        let mut file=OpenOptions::new().write(true).create_new(true).mode(0o600).open(&destination).map_err(|e|e.to_string())?;
        let mut buffer=[0;65536];loop{canceled()?;let n=entry.read(&mut buffer).map_err(|e|e.to_string())?;if n==0{break;}file.write_all(&buffer[..n]).map_err(|e|e.to_string())?;}
        let executable=entry.header().mode().map_err(|e|e.to_string())?&0o111!=0;
        file.set_permissions(fs::Permissions::from_mode(if executable{0o755}else{0o644})).map_err(|e|e.to_string())?;
    }
    // Files are written before links, so no archive link can redirect an extraction write.
    let symbolic=links.iter().filter(|(_,_,_,hard)|!*hard).map(|(p,_,_,_)|p.clone()).collect::<Vec<_>>();
    for (path,link,resolved,hard) in links{canceled()?;real_parents(stage,&path)?;if hard{let source=stage.join(resolved);let m=fs::symlink_metadata(&source).map_err(|e|e.to_string())?;if !m.is_file()||m.file_type().is_symlink(){return Err("Invalid archive hard link".into());}fs::hard_link(source,stage.join(path)).map_err(|e|e.to_string())?;}else{std::os::unix::fs::symlink(link,stage.join(path)).map_err(|e|e.to_string())?;}}
    let root=root.ok_or("Archive is empty")?;let result=stage.join(root);
    let canonical_root=fs::canonicalize(&result).map_err(|e|e.to_string())?;
    for link in symbolic{let resolved=fs::canonicalize(stage.join(link)).map_err(|_|"Archive contains an unresolved symbolic link")?;if !resolved.starts_with(&canonical_root){return Err("Archive link resolves outside Proton".into());}}
    for name in ["proton","compatibilitytool.vdf"]{let m=fs::symlink_metadata(result.join(name)).map_err(|_|format!("Proton archive is missing {name}"))?;if !m.is_file()||m.file_type().is_symlink(){return Err("Invalid Proton launcher or manifest".into());}}
    if fs::metadata(result.join("proton")).map_err(|e|e.to_string())?.permissions().mode()&0o111==0{return Err("Proton launcher is not executable".into());}
    crate::auto_select::internal_id("GE-Proton",&result)?;Ok(result)
}
struct Staging(PathBuf);
impl Drop for Staging{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
fn commit(source:&Path,destination:&Path)->Result<(),String>{
    use std::os::unix::ffi::OsStrExt;
    let a=std::ffi::CString::new(source.as_os_str().as_bytes()).map_err(|e|e.to_string())?;let b=std::ffi::CString::new(destination.as_os_str().as_bytes()).map_err(|e|e.to_string())?;
    // NOREPLACE makes the final installation atomic without overwriting a concurrent install.
    if unsafe{libc::renameat2(libc::AT_FDCWD,a.as_ptr(),libc::AT_FDCWD,b.as_ptr(),libc::RENAME_NOREPLACE)}!=0{return Err(format!("Could not install without replacing an existing version: {}",std::io::Error::last_os_error()));}Ok(())
}
async fn install(tag:&str,root:&str)->Result<(),String>{
    let bytes=small(&format!("{API}/tags/{tag}"),1024*1024).await?;
    let release=release_from(serde_json::from_slice(&bytes).map_err(|e|e.to_string())?).ok_or("No verified x86_64 release assets available")?;
    if release.tag!=tag{return Err("Release identity changed".into());}canceled()?;
    let expected=expected_hash(&small(&release.checksum_url,4096).await?,&release.name)?;
    let target=target(root)?;let name=release.name.trim_end_matches(".tar.gz");let destination=target.join(name);
    if fs::symlink_metadata(&destination).is_ok(){return Err("This version already exists. It was not replaced.".into());}
    let nonce=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
    let stage=Staging(target.join(format!(".proton-medic-{}-{nonce}",std::process::id())));fs::create_dir(&stage.0).map_err(|e|e.to_string())?;fs::set_permissions(&stage.0,fs::Permissions::from_mode(0o700)).map_err(|e|e.to_string())?;
    let archive=stage.0.join("download.tar.gz");let mut file=OpenOptions::new().create_new(true).write(true).mode(0o600).open(&archive).map_err(|e|e.to_string())?;
    update("downloading",tag,0,release.size,"Downloading from the official GE-Proton release.");
    let mut response=client()?.get(&release.url).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?;
    let mut hash=Sha512::new();let mut count=0u64;
    while let Some(chunk)=response.chunk().await.map_err(|e|e.to_string())?{canceled()?;count+=chunk.len() as u64;if count>release.size||count>MAX_DOWNLOAD{return Err("Download exceeds expected archive size".into());}file.write_all(&chunk).map_err(|e|e.to_string())?;hash.update(&chunk);update("downloading",tag,count,release.size,"Downloading from the official GE-Proton release.");}
    file.sync_all().map_err(|e|e.to_string())?;drop(file);canceled()?;
    if count!=release.size||format!("{:x}",hash.finalize())!=expected{return Err("Proton checksum verification failed. Nothing was installed.".into());}
    update("extracting",tag,count,release.size,"Checksum verified. Extracting Proton…");
    let unpack=stage.0.join("unpacked");fs::create_dir(&unpack).map_err(|e|e.to_string())?;
    let source=extract(flate2::read::GzDecoder::new(File::open(archive).map_err(|e|e.to_string())?),&unpack)?;canceled()?;
    OpenOptions::new().write(true).create_new(true).mode(0o600).open(source.join(".proton-medic-managed")).and_then(|mut f|f.write_all(tag.as_bytes())).map_err(|e|e.to_string())?;
    commit(&source,&destination)?;inventory::refresh();Ok(())
}
pub fn start(tag:String,root:String,confirmed:bool)->Result<(),String>{
    if !confirmed{return Err("Approve the Proton download first".into());}if !safe_name(&tag)||!tag.starts_with("GE-Proton"){return Err("Invalid Proton release".into());}
    if BUSY.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err(){return Err("A Proton installation is already in progress".into());}
    CANCEL.store(false,Ordering::Release);update("preparing",&tag,0,0,"Checking release metadata…");
    std::thread::spawn(move||{let result=(||{let _activity=crate::app_update::activity()?;tauri::async_runtime::block_on(install(&tag,&root))})();
        match result{Ok(())=>update("installed",&tag,0,0,"Installed. Restart Steam to load the new version; then choose it for a game."),Err(e)=>update(if CANCEL.load(Ordering::Acquire){"canceled"}else{"failed"},&tag,0,0,&e)}BUSY.store(false,Ordering::Release);
    });Ok(())
}
pub fn umu()->Option<PathBuf>{std::env::var_os("PATH").and_then(|p|std::env::split_paths(&p).filter(|p|p.is_absolute()).find_map(|p|{let p=p.join("umu-run");fs::metadata(&p).ok().filter(|m|m.is_file()&&m.permissions().mode()&0o111!=0).map(|_|p)}))}

#[cfg(test)] mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {fn new()->Self{let p=std::env::temp_dir().join(format!("pm-proton-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&p).unwrap();Self(p)}}
    impl Drop for Fixture {fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
    fn archive(extra:impl FnOnce(&mut tar::Builder<Vec<u8>>))->Vec<u8>{
        let mut tar=tar::Builder::new(vec![]);
        for (name,data,mode) in [("GE-Test/proton","#!/bin/sh\nexit 0\n",0o755),("GE-Test/compatibilitytool.vdf","\"compatibilitytools\" { \"compat_tools\" { \"GE-Test\" { \"install_path\" \".\" } } }",0o644)]{
            let mut h=tar::Header::new_gnu();h.set_size(data.len() as u64);h.set_mode(mode);h.set_cksum();tar.append_data(&mut h,name,data.as_bytes()).unwrap();
        }extra(&mut tar);tar.into_inner().unwrap()
    }
    fn link(tar:&mut tar::Builder<Vec<u8>>,name:&str,target:&str){let mut h=tar::Header::new_gnu();h.set_entry_type(tar::EntryType::Symlink);h.set_size(0);h.set_mode(0o777);h.set_cksum();tar.append_link(&mut h,name,target).unwrap();}
    #[test] fn archive_installs_files_and_safe_relative_links_without_execution(){
        let f=Fixture::new();let bytes=archive(|t|link(t,"GE-Test/alias","proton"));let root=extract(&bytes[..],&f.0).unwrap();
        assert!(root.join("proton").is_file());assert_eq!(fs::read_link(root.join("alias")).unwrap(),PathBuf::from("proton"));assert_eq!(fs::metadata(root.join("proton")).unwrap().permissions().mode()&0o777,0o755);
    }
    #[test] fn archive_rejects_escaping_absolute_and_cyclic_links(){
        for target in ["../../outside","/etc/passwd","alias"]{let f=Fixture::new();let bytes=archive(|t|link(t,"GE-Test/alias",target));assert!(extract(&bytes[..],&f.0).is_err(),"{target}");}
    }
    #[test] fn archive_rejects_link_parent_writes_and_duplicate_files(){
        let f=Fixture::new();let bytes=archive(|t|{link(t,"GE-Test/sub","proton");link(t,"GE-Test/sub/child","../proton");});assert!(extract(&bytes[..],&f.0).is_err());
        let f=Fixture::new();let bytes=archive(|t|{let mut h=tar::Header::new_gnu();h.set_size(1);h.set_mode(0o7777);h.set_cksum();t.append_data(&mut h,"GE-Test/proton",&b"x"[..]).unwrap();});assert!(extract(&bytes[..],&f.0).is_err());
    }
    #[test] fn archive_requires_valid_proton_manifest_and_launcher(){let f=Fixture::new();let mut t=tar::Builder::new(vec![]);let mut h=tar::Header::new_gnu();h.set_size(0);h.set_mode(0o644);h.set_cksum();t.append_data(&mut h,"empty/file",&b""[..]).unwrap();assert!(extract(&t.into_inner().unwrap()[..],&f.0).is_err());}
    #[test] fn paths_cannot_escape_or_use_parent_components(){for p in ["/tmp/file","../file","GE/../file",""]{assert!(relative(Path::new(p)).is_err());}assert_eq!(relative(Path::new("./GE/file")).unwrap(),PathBuf::from("GE/file"));}
    #[test] fn checksum_must_identify_the_selected_archive(){let hash="a".repeat(128);assert_eq!(expected_hash(format!("{hash}  GE.tar.gz\n").as_bytes(),"GE.tar.gz").unwrap(),hash);assert!(expected_hash(format!("{hash}  other.tar.gz").as_bytes(),"GE.tar.gz").is_err());assert!(expected_hash(b"broken GE.tar.gz","GE.tar.gz").is_err());}
    #[test] fn release_catalog_only_accepts_official_matching_architecture_assets(){
        fn raw(url:&str)->ApiRelease{serde_json::from_value(serde_json::json!({"tag_name":"GE-Proton11-11","draft":false,"prerelease":false,"assets":[{"name":"GE-Proton11-11-x86_64.tar.gz","size":100,"browser_download_url":url},{"name":"GE-Proton11-11-x86_64.sha512sum","size":160,"browser_download_url":format!("{ASSETS}GE-Proton11-11/GE-Proton11-11-x86_64.sha512sum")}]})).unwrap()}
        let url=format!("{ASSETS}GE-Proton11-11/GE-Proton11-11-x86_64.tar.gz");assert!(release_from(raw(&url)).is_some());assert!(release_from(raw("https://example.com/GE.tar.gz")).is_none());let mut other=raw(&url);other.assets[0].name="GE-Proton11-11-aarch64.tar.gz".into();assert!(release_from(other).is_none());let mut pre=raw(&url);pre.prerelease=true;assert!(release_from(pre).is_none());
    }
    #[test] fn final_install_does_not_replace_an_existing_version(){let f=Fixture::new();let a=f.0.join("new");let b=f.0.join("old");fs::create_dir(&a).unwrap();fs::create_dir(&b).unwrap();fs::write(b.join("keep"),"original").unwrap();assert!(commit(&a,&b).is_err());assert_eq!(fs::read_to_string(b.join("keep")).unwrap(),"original");let c=f.0.join("installed");commit(&a,&c).unwrap();assert!(c.is_dir());}
    #[test] fn unapproved_download_is_rejected_before_network_or_disk_changes(){assert!(start("GE-Proton11-11".into(),"/unused".into(),false).is_err());assert!(start("../bad".into(),"/unused".into(),true).is_err());}
    #[test]
    #[ignore = "Downloads the pinned official release; run explicitly in Linux validation"]
    fn official_ge_release_installs_with_verification(){
        let f=Fixture::new();let steam=f.0.join(".local/share/Steam");fs::create_dir_all(steam.join("steamapps")).unwrap();
        let previous=std::env::var_os("HOME");std::env::set_var("HOME",&f.0);
        let result=tauri::async_runtime::block_on(install("GE-Proton11-11",steam.to_str().unwrap()));
        if let Some(home)=previous{std::env::set_var("HOME",home);}else{std::env::remove_var("HOME");}
        result.unwrap();let installed=steam.join("compatibilitytools.d/GE-Proton11-11-x86_64");assert!(installed.join("proton").is_file());assert!(installed.join(".proton-medic-managed").is_file());
        assert_eq!(crate::auto_select::internal_id("GE-Proton",&installed).unwrap(),"GE-Proton11-11-x86_64");
        assert_eq!(fs::read_dir(steam.join("compatibilitytools.d")).unwrap().count(),1);
        println!("Official GE-Proton download, SHA-512 verification, safe extraction and atomic installation passed.");
    }
}
