use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::{fs, io};
use std::hash::{Hash, Hasher};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use filetime::{FileTime, set_file_mtime};

pub(crate) fn apply_diffs_source_to_target_with_prints<'a, I>(source_base_path: &str, target_base_path: &str, diffs: I) where I: Iterator<Item= &'a Difference>+Clone {
    let mut to_buf = PathBuf::new();
    for d in diffs {
        if d.p_source.is_some() && d.p_target.is_some() {
            let psu = d.p_source.as_ref().unwrap();
            let from = &psu.path;
            let to = &d.p_target.as_ref().unwrap().path;
            println!("Replacing file/directory...:\n    '{from}' -> {to}");
            let err = copy_file_or_dir_with_prints(psu, &from, &to);
            match err {
                Ok(len) => println!("Successfully replaced file/directory: \n    '{from}' -> {to}\n    {len} bytes written"),
                Err(e) => println!("Error replacing file/directory: \n    '{from}' -> {to}\n    {e}")
            }
        } else if d.p_source.is_some() && d.p_target.is_none() {
            let psu = d.p_source.as_ref().unwrap();
            let from = &psu.path;
            to_buf.clear();
            to_buf.push(target_base_path);
            to_buf.push(&from[source_base_path.len() + if source_base_path.starts_with("/") {1} else {1}..]);
            println!("Copying file/directory...:\n    '{from}' -> {}", to_buf.to_str().unwrap());
            let err = copy_file_or_dir_with_prints(psu, &from, &to_buf.to_str().unwrap());
            match err {
                Ok(len) => println!("Successfully copied file/directory: \n    '{from}' -> {}\n    {len} bytes written", to_buf.to_str().unwrap()),
                Err(e) => println!("Error copied file/directory: \n    '{from}' -> {}\n    {e}", to_buf.to_str().unwrap())
            }
        } else if d.p_source.is_none() && d.p_target.is_some() {
            let ptu = d.p_target.as_ref().unwrap();
            let pt_path = &ptu.path;
            let err = if ptu.is_dir() {
                println!("Removing directory...: '{pt_path}'");
                fs::remove_dir_all(&pt_path)
            } else {
                println!("Removing file...: '{pt_path}'");
                fs::remove_file(&pt_path)
            };
            match err {
                Ok(_)        => println!("Successfully removed file/directory: ’{pt_path}’"),
                Err(e) => println!("Error removing file/directory: ’{pt_path}’\n    {e}")
            }
        }
    }
}

fn copy_file_or_dir_with_prints(psu: &AnnotatedPath, from: &str, to: &str)-> io::Result<u64> {
    return if psu.is_dir() {
        let mut byte_counter = 0u64;
        match fs::create_dir(&to) {
            Ok(_) => {}
            Err(e) => {
                match e.kind() {
                    ErrorKind::AlreadyExists => {}
                    _ => {return Err(e);}
                }
            }
        }

        let mut target_path = PathBuf::new();
        for entry in walkdir::WalkDir::new(&from)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            target_path.clear();
            target_path.push(to);
            target_path.push(entry.path().strip_prefix(from).unwrap());

            let source_metadata = match entry.metadata() {
                Ok(md) => {md}
                Err(e) => { return Err(io::Error::from(e)); }
            };

            if source_metadata.is_dir() {
                match fs::create_dir(&target_path) {
                    Ok(_) => {}
                    Err(e) => { return Err(e); }
                };
                match fs::set_permissions(&target_path, source_metadata.permissions()) {
                    Ok(_) => {}
                    Err(e) => { return Err(e); }
                }
            } else {
                let source_modified = match source_metadata.modified() {
                    Ok(m) => { m }
                    Err(e) => { return Err(e); }
                };

                match copy_file_update_time(source_modified, entry.path(), &target_path) {
                    Ok(bytes) => { byte_counter += bytes; }
                    Err(e) => { return Err(e); }
                };
            }
        }
        Ok(byte_counter)
    } else {
        return copy_file_update_time(psu.modified(), from, to);
    }
}
fn copy_file_update_time<P: AsRef<Path>, Q: AsRef<Path>>(from_modified: SystemTime, from: P, to: Q) -> io::Result<u64> {
    return match fs::copy(&from, &to) {
        Ok(bytes) => {
            match set_file_mtime(&to, FileTime::from(from_modified)) {
                Ok(_) => {}
                Err(e) => { return Err(e); }
            }
            Ok(bytes)
        }
        Err(e) => { Err(e) }
    };
}



#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub(crate) struct Difference {
    pub(crate) p_source: Option<AnnotatedPath>,
    pub(crate) p_target: Option<AnnotatedPath>
}

impl Difference {
    pub(crate) fn describe(&self) -> String {
        let file_name = self.file_name();

        if self.p_source.is_some() && self.p_target.is_some() {
            //always a file
            return format!("MODIFIED ({}): {}[{}]", if self.p_source.as_ref().unwrap().modified() > self.p_target.as_ref().unwrap().modified() { "source is newer" } else { "backup is newer" }, if self.is_dir() { "DIR" } else { "FILE" }, file_name)
        } else if self.p_source.is_some() && self.p_target.is_none() {
            return format!("NEW in source (or deleted in backup): {}[{}]", if self.is_dir() { "DIR" } else { "FILE" }, file_name);
        } else if self.p_source.is_none() && self.p_target.is_some() {
            return format!("DELETED in source (or new in backup): {}[{}]", if self.is_dir() { "DIR" } else { "FILE" }, file_name);
        } else {
            panic!("impossible, this is a bug")
        }
    }
    pub(crate) fn describe_short(&self) -> String {
        let file_name = self.file_name();

        if self.p_source.is_some() && self.p_target.is_some() {
            //always a file
            return format!("MODIFIED ({}): {}[\"{}\"]", if self.p_source.as_ref().unwrap().modified() > self.p_target.as_ref().unwrap().modified() { "source new" } else { "backup new" }, if self.is_dir() { "DIR" } else { "FILE" }, file_name)
        } else if self.p_source.is_some() && self.p_target.is_none() {
            return format!("NEW: {}[\"{}\"]", if self.is_dir() { "DIR" } else { "FILE" }, file_name);
        } else if self.p_source.is_none() && self.p_target.is_some() {
            return format!("DELETED: {}[\"{}\"]", if self.is_dir() { "DIR" } else { "FILE" }, file_name);
        } else {
            panic!("impossible, this is a bug")
        }
    }

    pub(crate) fn get_directory_path(&self, source_path_len: usize, target_path_len: usize) -> &str {
        let some_full_path = &(if self.p_source.is_some() {&self.p_source} else {&self.p_target}.as_ref().unwrap()).path;
        return &some_full_path[if self.p_source.is_some() { source_path_len } else { target_path_len }..some_full_path.len() - self.file_name().len()];
    }
}

impl Difference {
    pub(crate) fn ps_modified(&self) -> SystemTime {
        return self.p_source.as_ref().unwrap().modified()
    }
    pub(crate) fn pt_modified(&self) -> SystemTime {
        return self.p_target.as_ref().unwrap().modified()
    }
    pub(crate) fn ps_path(&self) -> &str {
        return &self.p_source.as_ref().unwrap().path
    }
    pub(crate) fn pt_path(&self) -> &str {
        return &self.p_target.as_ref().unwrap().path
    }
    pub(crate) fn is_dir(&self) -> bool {
        if self.p_source.is_some() && self.p_target.is_some() {
            let s_is_dir = self.p_source.as_ref().unwrap().is_dir();
            let t_is_dir = self.p_target.as_ref().unwrap().is_dir();
            if s_is_dir != t_is_dir {
                panic!("one is file and one is dir")
            }
            return s_is_dir;
        } else if self.p_source.is_some() && self.p_target.is_none() {
            return self.p_source.as_ref().unwrap().is_dir()
        } else if self.p_source.is_none() && self.p_target.is_some() {
            return self.p_target.as_ref().unwrap().is_dir()
        } else {
            panic!("both are none, never happens, bug")
        }
    }
    pub(crate) fn file_name(&self) -> &str {
        if self.p_source.is_some() /*&& self.p_target.is_some()*/ {
            //filename of both always the same
            return &*self.p_source.as_ref().unwrap().name;
            // } else if self.p_source.is_some() && self.p_target.is_none() {
            //     return &*self.p_source.as_ref().unwrap().name;
        } else if self.p_source.is_none() && self.p_target.is_some() {
            return &*self.p_target.as_ref().unwrap().name;
        } else {
            panic!("both are none, never happens, bug")
        }
    }
    // pub fn max_modified(&self) -> Option<SystemTime> {
    //     return if self.p_source.is_some() && self.p_target.is_some() && !self.p_source.as_ref().unwrap().is_dir() && !self.p_target.as_ref().unwrap().is_dir() {
    //         Some(self.ps_modified().max(self.pt_modified()))
    //     } else if self.p_source.is_some() && !self.p_source.as_ref().unwrap().is_dir() {
    //         Some(self.ps_modified())
    //     } else if self.p_target.is_some() && !self.p_target.as_ref().unwrap().is_dir() {
    //         Some(self.pt_modified())
    //     } else {
    //         None
    //     }
    // }
}




#[derive(Debug, Clone)]
pub(crate) struct AnnotatedPath {
    pub(crate) path: String,
    name: String,
    modified: Option<SystemTime>
}
impl AnnotatedPath {
    pub fn is_dir(&self) -> bool {
        return self.modified.is_none()
    }
    pub fn modified(&self) -> SystemTime {
        return self.modified.expect("cannot query modified for directories for reasons of fs independence")
    }
}

impl Eq for AnnotatedPath {}
impl Hash for AnnotatedPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        return self.name.hash(state);
    }
}

impl PartialEq<Self> for AnnotatedPath {
    fn eq(&self, other: &Self) -> bool {
        return self.name == other.name
    }
}
impl PartialOrd<Self> for AnnotatedPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        return Some(self.cmp(&other));
    }
}
impl Ord for AnnotatedPath {
    fn cmp(&self, other: &Self) -> Ordering {
        return self.path.cmp(&other.path)
    }
}



/// Will now analyse directories and verify that target directory does not contain any files that\n    \
///               are newer than their expression in the source and\n    \
///               that target directory does not contain any files that don't exist in source,\n    \
///               but are newer than the last common modification date (assumed time of last synchronization).
/// Returns list of files that are assumed newer in target directory ("problems").
pub(crate) fn verify_source_fully_newer_than_target(differences: &Vec<Difference>) -> HashMap<Difference, String> {
    let mut problems = HashMap::new();

    if differences.is_empty() {
        return problems;
    }

    let mut assumed_time_of_divergence = SystemTime::UNIX_EPOCH;
    for d in differences {
        if d.is_dir() {continue}
        if d.p_source.is_some() && d.p_target.is_some() {
            assumed_time_of_divergence = assumed_time_of_divergence.max(d.ps_modified());
        } else if d.p_source.is_none() && d.p_target.is_some() {
            //decision: is file new in target (problem) or deleted in source (no problem)
            //cannot be decided...
        }
    }

    for d in differences {
        if d.p_source.is_some() && d.p_target.is_some() {
            if !d.is_dir() && d.pt_modified() > d.ps_modified() {
                problems.insert(d.clone(), "NEWER in backup directory".to_string());
            }
        } else if d.p_source.is_none() && d.p_target.is_some() {
            if d.is_dir() {
                problems.insert(d.clone(), "Directory exists in backup directory, but NOT in source directory.".to_string());
            } else if d.pt_modified() >= assumed_time_of_divergence {
                problems.insert(d.clone(), "File exists in backup directory, but NOT in source directory and cannot be verified to be old.".to_string());
            }
        }
    }

    return problems
}



pub(crate) fn find_differences(source_dir: &str, target_dir: &str) -> Vec<Difference> {
    let mut collector = Vec::with_capacity(64);

    find_differences_rec(source_dir, target_dir, &mut collector);

    return collector
}

fn find_differences_rec(dir1: &str, dir2: &str, collector: &mut Vec<Difference>) {
    let dir1_set = list_paths(dir1);
    let dir2_set = list_paths(dir2);

    for f2 in &dir2_set {
        let f1o = dir1_set.get(f2);
        if f1o.is_none() {
            collector.push(Difference { p_source: None, p_target: Some(f2.clone()) });
        }
    }

    for f1 in &dir1_set {
        let f2o = dir2_set.get(f1);
        if f2o.is_some() {
            let f2 = f2o.unwrap();
            if f1.is_dir() && f2.is_dir() {
                find_differences_rec(&f1.path, &f2.path, collector);
            } else {
                if f1.is_dir() != f2.is_dir() || f1.modified != f2.modified {
                    collector.push(Difference { p_source: Some(f1.clone()), p_target: f2o.cloned()});
                }
            }
        } else {
            collector.push(Difference { p_source: Some(f1.clone()), p_target: None });
        }
    }
}


fn list_paths(dir: &str) -> HashSet<AnnotatedPath> {
    return match fs::read_dir(dir) {
        Ok(reader) => {
            let mut result = HashSet::new();
            for r in reader {
                let e = r.unwrap();
                let path = e.path().to_str().unwrap().to_string();
                let name = e.file_name().to_str().unwrap().to_string();
                let meta = e.metadata().unwrap();
                let is_dir = meta.is_dir();
                let modified = if is_dir {
                    None
                } else {
                    Some(meta.modified().unwrap())
                };
                result.insert(AnnotatedPath { path, name, modified });
            }
            result
        }
        Err(_) => { HashSet::with_capacity(0) }
    }
}
