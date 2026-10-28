//! Bounded, read-only artwork loading. Windows executables are never run.
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{fs::{self, File}, io::{Cursor, Read, Seek, SeekFrom}, path::{Path, PathBuf}};
const MAX_ICON: usize = 1024 * 1024;
fn u16le(b:&[u8],at:usize)->Option<u16>{Some(u16::from_le_bytes(b.get(at..at+2)?.try_into().ok()?))}
fn u32le(b:&[u8],at:usize)->Option<u32>{Some(u32::from_le_bytes(b.get(at..at+4)?.try_into().ok()?))}
pub fn data_uri(bytes:&[u8],mime:&str)->String{format!("data:{mime};base64,{}",STANDARD.encode(bytes))}
fn open_regular(path:&Path)->Option<File>{
    use std::os::unix::fs::OpenOptionsExt;
    let file=File::options().read(true).custom_flags(libc::O_NOFOLLOW|libc::O_NONBLOCK).open(path).ok()?;
    file.metadata().ok()?.is_file().then_some(file)
}
fn read_at(file:&mut File,len:u64,offset:u64,size:usize,limit:usize)->Option<Vec<u8>>{
    if size>limit || offset.checked_add(size as u64)?>len{return None;}
    let mut data=vec![0;size];file.seek(SeekFrom::Start(offset)).ok()?;file.read_exact(&mut data).ok()?;Some(data)
}
struct Section{rva:u32,size:u32,offset:u32}
struct Pe{file:File,len:u64,sections:Vec<Section>,resource:u32,size:u32}
struct Entry{id:Option<u32>,offset:u32,directory:bool}
impl Pe{
    fn open(path:&Path)->Option<Self>{
        let mut file=open_regular(path)?;let meta=file.metadata().ok()?;if !meta.is_file(){return None;}let len=meta.len();
        let dos=read_at(&mut file,len,0,64,64)?;if &dos[..2]!=b"MZ"{return None;}
        let nt=u32le(&dos,60)? as u64;
        let header=read_at(&mut file,len,nt,24,24)?;if &header[..4]!=b"PE\0\0"{return None;}
        let count=u16le(&header,6)? as usize;if count==0 || count>96{return None;}
        let optional_len=u16le(&header,20)? as usize;
        let optional=read_at(&mut file,len,nt.checked_add(24)?,optional_len,4096)?;
        let data=match u16le(&optional,0)?{0x10b=>96,0x20b=>112,_=>return None};
        if u32le(&optional,data-4)?<3{return None;}
        let resource=u32le(&optional,data+16)?;let size=u32le(&optional,data+20)?;
        if resource==0 || size<16{return None;}
        let table=read_at(&mut file,len,nt.checked_add(24+optional_len as u64)?,count*40,96*40)?;
        let mut sections=vec![];
        for section in table.chunks_exact(40){sections.push(Section{rva:u32le(section,12)?,size:u32le(section,16)?,offset:u32le(section,20)?});}
        Some(Self{file,len,sections,resource,size})
    }
    fn read_rva(&mut self,rva:u32,size:usize,limit:usize)->Option<Vec<u8>>{
        let section=self.sections.iter().find(|s|rva>=s.rva && (rva as u64+size as u64)<=s.rva as u64+s.size as u64)?;
        let offset=section.offset as u64+(rva-section.rva) as u64;
        read_at(&mut self.file,self.len,offset,size,limit)
    }
    fn resource_bytes(&mut self,offset:u32,size:usize,limit:usize)->Option<Vec<u8>>{
        if offset as u64+size as u64>self.size as u64{return None;}
        self.read_rva(self.resource.checked_add(offset)?,size,limit)
    }
    fn entries(&mut self,offset:u32)->Option<Vec<Entry>>{
        let header=self.resource_bytes(offset,16,16)?;
        let count=u16le(&header,12)? as usize+u16le(&header,14)? as usize;if count>256{return None;}
        let entries=self.resource_bytes(offset.checked_add(16)?,count*8,256*8)?;
        Some(entries.chunks_exact(8).map(|e|{let name=u32le(e,0).unwrap();let target=u32le(e,4).unwrap();Entry{id:(name&0x8000_0000==0).then_some(name),offset:target&0x7fff_ffff,directory:target&0x8000_0000!=0}}).collect())
    }
    fn bytes(&mut self,entry:&Entry,limit:usize)->Option<Vec<u8>>{
        if entry.directory{return None;}
        let data=self.resource_bytes(entry.offset,16,16)?;
        self.read_rva(u32le(&data,0)?,u32le(&data,4)? as usize,limit)
    }
    fn image(&mut self,id:u32)->Option<Vec<u8>>{
        let icons=self.entries(0)?.into_iter().find(|e|e.id==Some(3)&&e.directory)?;
        let name=self.entries(icons.offset)?.into_iter().find(|e|e.id==Some(id)&&e.directory)?;
        let lang=self.entries(name.offset)?.into_iter().find(|e|!e.directory)?;
        self.bytes(&lang,MAX_ICON)
    }
}
fn valid_dimensions(bytes:&[u8])->bool{
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n"){
        if bytes.get(12..16)!=Some(b"IHDR"){return false;}
        let Some(width)=bytes.get(16..20).and_then(|s|s.try_into().ok()).map(u32::from_be_bytes) else{return false};
        let Some(height)=bytes.get(20..24).and_then(|s|s.try_into().ok()).map(u32::from_be_bytes) else{return false};
        (1..=256).contains(&width)&&(1..=256).contains(&height)
    }else{
        matches!(u32le(bytes,0),Some(40|108|124)) && u32le(bytes,4).is_some_and(|w|(1..=256).contains(&w)) && u32le(bytes,8).is_some_and(|h|(2..=512).contains(&h)&&h%2==0)
    }
}
fn decode_entry(entry:&[u8],image:Vec<u8>)->Option<Vec<u8>>{
    if !valid_dimensions(&image){return None;}
    let mut ico=vec![0,0,1,0,1,0];ico.extend_from_slice(entry.get(..8)?);ico.extend_from_slice(&(image.len() as u32).to_le_bytes());ico.extend_from_slice(&22u32.to_le_bytes());ico.extend(image);
    let directory=ico::IconDir::read(Cursor::new(ico)).ok()?;
    let decoded=directory.entries().first()?.decode().ok()?;
    if decoded.width()>256 || decoded.height()>256{return None;}
    let mut png=vec![];decoded.write_png(&mut png).ok()?;Some(png)
}
pub fn executable_png(path:&Path)->Option<Vec<u8>>{
    let mut pe=Pe::open(path)?;
    let group_type=pe.entries(0)?.into_iter().find(|e|e.id==Some(14)&&e.directory)?;
    let mut budget=8;
    for group in pe.entries(group_type.offset)?.into_iter().filter(|e|e.directory).take(8){
        let Some(language)=pe.entries(group.offset).and_then(|v|v.into_iter().find(|e|!e.directory)) else{continue};
        let Some(data)=pe.bytes(&language,1024) else{continue};
        if u16le(&data,0)!=Some(0) || u16le(&data,2)!=Some(1){continue;}
        let count=u16le(&data,4)? as usize;if count==0 || count>64 || data.len()<6+14*count{continue;}
        let mut entries:Vec<_>=data[6..6+14*count].chunks_exact(14).collect();
        entries.sort_by_key(|e|std::cmp::Reverse((if e[0]==0{256}else{e[0] as u32})*(if e[1]==0{256}else{e[1] as u32})));
        for entry in entries{
            if budget==0{return None;}budget-=1;
            let Some(image)=pe.image(u16le(entry,12)? as u32) else{continue};
            if let Some(png)=decode_entry(entry,image){return Some(png);}
        }
    }
    None
}
fn local_image(path:&Path)->Option<String>{
    let mut file=open_regular(path)?;let meta=file.metadata().ok()?;
    if !meta.is_file() || meta.len()>MAX_ICON as u64{return None;}
    let mut bytes=vec![];file.by_ref().take(MAX_ICON as u64+1).read_to_end(&mut bytes).ok()?;
    let mime=if bytes.starts_with(b"\x89PNG\r\n\x1a\n"){"image/png"}else if bytes.starts_with(b"\xff\xd8\xff"){"image/jpeg"}else{return None};
    Some(data_uri(&bytes,mime))
}
fn steam_art(root:&Path,app_id:&str)->Option<String>{
    let cache=root.join("appcache/librarycache");
    for name in [format!("{app_id}_icon.jpg"),format!("{app_id}_icon.png")]{if let Some(icon)=local_image(&cache.join(name)){return Some(icon);}}
    let folder=cache.join(app_id);
    if let Ok(entries)=fs::read_dir(&folder){
        let mut icons:Vec<PathBuf>=entries.take(512).filter_map(Result::ok).map(|e|e.path()).filter(|p|p.file_stem().and_then(|s|s.to_str()).is_some_and(|s|s.len()==40&&s.bytes().all(|b|b.is_ascii_hexdigit()))).collect();icons.sort();
        for path in icons{if let Some(icon)=local_image(&path){return Some(icon);}}
    }
    for name in ["icon.png","icon.jpg","header.jpg"]{if let Some(icon)=local_image(&folder.join(name)){return Some(icon);}}
    None
}
#[derive(Default)]
struct SteamIndex{roots:Vec<PathBuf>,games:std::collections::HashSet<(String,String)>}
static STEAM_INDEX:std::sync::LazyLock<std::sync::RwLock<SteamIndex>>=std::sync::LazyLock::new(||std::sync::RwLock::new(SteamIndex::default()));
pub fn remember_steam(scan:&crate::scanner::ScanReport){
    if let Ok(mut index)=STEAM_INDEX.write(){
        index.roots=scan.installations.iter().map(|i|PathBuf::from(&i.path)).collect();
        index.games=scan.games.iter().map(|g|(g.app_id.clone(),g.library_path.clone())).collect();
    }
}
pub fn steam_icon(app_id:String,library_path:String)->Result<Option<String>,String>{
    let roots={
        let index=STEAM_INDEX.read().map_err(|_|"Steam artwork index unavailable")?;
        if !index.games.contains(&(app_id.clone(),library_path)){return Err("Choose a scanned Steam game".into());}
        index.roots.clone()
    };
    for root in roots{if let Some(icon)=steam_art(&root,&app_id){return Ok(Some(icon));}}
    Ok(None)
}

#[cfg(test)]
mod tests{
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture{fn new()->Self{let p=std::env::temp_dir().join(format!("pm-icons-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&p).unwrap();Self(p)}}
    impl Drop for Fixture{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
    fn put16(b:&mut[u8],p:usize,v:u16){b[p..p+2].copy_from_slice(&v.to_le_bytes());}
    fn put32(b:&mut[u8],p:usize,v:u32){b[p..p+4].copy_from_slice(&v.to_le_bytes());}
    fn png()->Vec<u8>{let image=ico::IconImage::from_rgba_data(2,2,[100u8,180,240,255].repeat(4));let mut bytes=vec![];image.write_png(&mut bytes).unwrap();bytes}
    fn pe(image:&[u8],plus:bool)->Vec<u8>{
        let optional=if plus{240}else{224};let mut b=vec![0;1024];b[..2].copy_from_slice(b"MZ");put32(&mut b,60,128);b[128..132].copy_from_slice(b"PE\0\0");put16(&mut b,134,1);put16(&mut b,148,optional as u16);
        put16(&mut b,152,if plus{0x20b}else{0x10b});let data=152+if plus{112}else{96};put32(&mut b,data-4,16);put32(&mut b,data+16,0x1000);put32(&mut b,data+20,512);
        let section=152+optional;put32(&mut b,section+12,0x1000);put32(&mut b,section+16,512);put32(&mut b,section+20,512);
        let r=&mut b[512..];put16(r,14,2);put32(r,16,3);put32(r,20,0x8000_0020);put32(r,24,14);put32(r,28,0x8000_0050);
        for (dir,target) in [(32,56),(80,104)]{put16(r,dir+14,1);put32(r,dir+16,1);put32(r,dir+20,0x8000_0000|target as u32);}
        for (dir,target) in [(56,128),(104,144)]{put16(r,dir+14,1);put32(r,dir+16,1033);put32(r,dir+20,target as u32);}
        let group=160+image.len();put32(r,128,0x1000+160);put32(r,132,image.len() as u32);put32(r,144,0x1000+group as u32);put32(r,148,20);
        r[160..160+image.len()].copy_from_slice(image);put16(r,group+2,1);put16(r,group+4,1);r[group+6]=2;r[group+7]=2;put16(r,group+10,1);put16(r,group+12,32);put32(r,group+14,image.len() as u32);put16(r,group+18,1);b
    }
    #[test] fn extracts_png_resources_from_pe32_and_pe64(){
        let f=Fixture::new();let path=f.0.join("game.exe");
        for plus in [false,true]{fs::write(&path,pe(&png(),plus)).unwrap();let out=executable_png(&path).unwrap();let image=ico::IconImage::read_png(Cursor::new(out)).unwrap();assert_eq!((image.width(),image.height()),(2,2));assert_eq!(&image.rgba_data()[..4],&[100,180,240,255]);}
    }
    #[test] fn converts_legacy_dib_resources_to_png(){
        let f=Fixture::new();let path=f.0.join("game.exe");let mut dib=vec![0;64];put32(&mut dib,0,40);put32(&mut dib,4,2);put32(&mut dib,8,4);put16(&mut dib,12,1);put16(&mut dib,14,32);
        for pixel in dib[40..56].chunks_exact_mut(4){pixel.copy_from_slice(&[240,180,100,255]);}
        fs::write(&path,pe(&dib,true)).unwrap();assert!(executable_png(&path).unwrap().starts_with(b"\x89PNG"));
    }
    #[test] fn rejects_truncated_resources_cycles_and_oversized_images(){
        let f=Fixture::new();let path=f.0.join("bad.exe");let original=pe(&png(),true);
        for length in [0,1,63,128,200,512,650]{fs::write(&path,&original[..length]).unwrap();assert!(executable_png(&path).is_none());}
        let mut cyclic=original.clone();put32(&mut cyclic,512+28,0x8000_0000);fs::write(&path,cyclic).unwrap();assert!(executable_png(&path).is_none());
        let mut oversized=original;put32(&mut oversized,512+132,u32::MAX);fs::write(&path,oversized).unwrap();assert!(executable_png(&path).is_none());
        let mut huge=png();huge[16..20].copy_from_slice(&u32::MAX.to_be_bytes());assert!(!valid_dimensions(&huge));
    }
    #[test] fn large_sparse_executables_do_not_require_reading_the_whole_file(){
        let f=Fixture::new();let path=f.0.join("large.exe");fs::write(&path,pe(&png(),true)).unwrap();File::options().write(true).open(&path).unwrap().set_len(1024*1024*1024).unwrap();assert!(executable_png(&path).is_some());
    }
    #[test] fn reads_steam_cached_icons_without_network(){
        let f=Fixture::new();let folder=f.0.join("appcache/librarycache/123");fs::create_dir_all(&folder).unwrap();fs::write(folder.join("a".repeat(40)+".jpg"),b"\xff\xd8\xfffixture").unwrap();
        assert!(steam_art(&f.0,"123").unwrap().starts_with("data:image/jpeg;base64,"));assert!(steam_art(&f.0,"999").is_none());
    }
    #[test] fn malformed_images_and_missing_executables_have_a_fallback(){
        let f=Fixture::new();let file=f.0.join("bad.exe");fs::write(&file,b"not an executable").unwrap();assert!(executable_png(&file).is_none());assert!(executable_png(&f.0.join("missing.exe")).is_none());assert!(local_image(&file).is_none());
    }
}
