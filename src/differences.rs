use std::cmp::Ordering;
use std::collections::{HashSet, LinkedList};
use std::fs;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;
use filetime::{FileTime, set_file_mtime};


pub(crate) fn apply_diffs_source_to_target_with_prints<'a, I>(source_base_path: &str, target_base_path: &str, diffs: I) where I: Iterator<Item= &'a Difference>+Clone {
    for d in diffs {
        if d.p_source.is_some() && d.p_target.is_some() {
            let psu = d.p_source.as_ref().unwrap();
            let from = &psu.path;
            let to = &d.p_target.as_ref().unwrap().path;
            copy_file_or_dir_with_prints(psu, &from, &to);
        } else if d.p_source.is_some() && d.p_target.is_none() {
            let psu = d.p_source.as_ref().unwrap();
            let from = &psu.path;
            let to = format!("{}/{}", target_base_path, &from[source_base_path.len()..]);
            copy_file_or_dir_with_prints(psu, &from, &to);
        } else if d.p_source.is_none() && d.p_target.is_some() {
            let ptu = d.p_target.as_ref().unwrap();
            let ptpath = &ptu.path;
            let err = if ptu.is_dir() {
                fs::remove_dir_all(&ptpath)
            } else {
                fs::remove_file(&ptpath)
            };
            match err {
                Ok(_) => {
                    println!("Successfully removed file/directory:\n    {ptpath}")
                }
                Err(e) => {
                    println!("Error removing file/directory (already deleted?):\n    {e}\n    {ptpath}")
                }
            }
        }
    }
}

fn copy_file_or_dir_with_prints(psu: &AnnotatedPath, from: &str, to: &str) {
    if psu.is_dir() {
        match fs::create_dir(&to) {
            Ok(_) => {
                println!("Successfully copied directory.\n    {from} -> {to}");
            }
            Err(e) => {
                println!("Error copying directory:\n    {e}\n    {from} -> {to}")
            }
        }
    } else {
        match fs::copy(&from, &to) {
            Ok(bytes) => {
                println!("Successfully wrote {bytes} bytes.\n    {from} -> {to}");
                set_file_mtime(&to, FileTime::from(psu.modified())).unwrap();
            }
            Err(e) => {
                println!("Error copying file:\n    {e}\n    {from} -> {to}")
            }
        }
    }
}


#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub(crate) struct Difference {
    pub(crate) p_source: Option<AnnotatedPath>,
    pub(crate) p_target: Option<AnnotatedPath>
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
            panic!("both are none")
        }
    }
    pub fn max_modified(&self) -> Option<SystemTime> {
        return if self.p_source.is_some() && self.p_target.is_some() && !self.p_source.as_ref().unwrap().is_dir() && !self.p_target.as_ref().unwrap().is_dir() {
            Some(self.ps_modified().max(self.pt_modified()))
        } else if self.p_source.is_some() && !self.p_source.as_ref().unwrap().is_dir() {
            Some(self.ps_modified())
        } else if self.p_target.is_some() && !self.p_target.as_ref().unwrap().is_dir() {
            Some(self.pt_modified())
        } else {
            None
        }
    }
}




#[derive(Debug, Clone)]
struct AnnotatedPath {
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
pub(crate) fn verify_source_fully_newer_than_target(most_recent_modified_in_source: SystemTime, differences: &LinkedList<Difference>) -> Vec<(Difference, String)> {
    let mut problems = Vec::new();

    if differences.is_empty() {
        return problems;
    }

    let mut assumed_time_of_divergence: Option<SystemTime> = None;
    for d in differences {
        if d.p_source.is_some() && d.p_target.is_some() {
            if assumed_time_of_divergence.is_some() {
                assumed_time_of_divergence = Some(assumed_time_of_divergence.unwrap().max(d.pt_modified()));
            } else {
                assumed_time_of_divergence = Some(d.pt_modified());
            }
        }
    }
    if assumed_time_of_divergence == None {
        assumed_time_of_divergence = Some(most_recent_modified_in_source);
    }


    println!("assumed_time_of_divergence: {:?}", assumed_time_of_divergence);

    for d in differences {
        if d.p_source.is_some() && d.p_target.is_some() {
            if d.pt_modified() > d.ps_modified() {
                let mut str = "Path '".to_string();
                str.push_str(d.pt_path());
                str.push_str("' NEWER in backup directory.");
                problems.push((d.clone(), str));
            }
        } else if d.p_source.is_none() && d.p_target.is_some() {
            if !d.is_dir() && d.pt_modified() >= assumed_time_of_divergence.unwrap() {
                let mut str = "Path '".to_string();
                str.push_str(d.pt_path());
                str.push_str("' ADDED after last backup in backup directory.");
                problems.push((d.clone(), str));
            }
        }
    }

    return problems
}



pub(crate) fn find_differences(source_dir: &str, target_dir: &str) -> (SystemTime, LinkedList<Difference>) {
    let mut collector = LinkedList::new();

    let most_recent_modified_in_source = find_differences_rec(Some(source_dir), Some(target_dir), &mut collector);

    return (most_recent_modified_in_source, collector)
}

fn find_differences_rec(dir1: Option<&str>, dir2: Option<&str>, mut collector: &mut LinkedList<Difference>) -> SystemTime {
    let dir1_set = list_paths(dir1);
    let dir2_set = list_paths(dir2);
    let mut most_recent_modified_in_source = SystemTime::UNIX_EPOCH;

    for f1 in &dir1_set {
        if !f1.is_dir() {
            most_recent_modified_in_source = most_recent_modified_in_source.max(f1.modified());
        }
        let f2o = dir2_set.get(f1);
        if f2o.is_some() {
            let f2 = f2o.unwrap();
            if f1.is_dir() && f2.is_dir() {
                let mrmis = find_differences_rec(Some(&f1.path), Some(&f2.path), collector);
                most_recent_modified_in_source = most_recent_modified_in_source.max(mrmis);
                if f1.is_dir() != f2.is_dir() || f1.modified != f2.modified {
                    collector.push_front(Difference { p_source: Some(f1.clone()), p_target: f2o.cloned()});
                }
            } else {
                if f1.is_dir() != f2.is_dir() || f1.modified != f2.modified {
                    collector.push_back(Difference { p_source: Some(f1.clone()), p_target: f2o.cloned()});
                }
            }
        } else {
            if f1.is_dir() {
                let mrmis = find_differences_rec(Some(&f1.path), None, collector);
                most_recent_modified_in_source = most_recent_modified_in_source.max(mrmis);
                collector.push_front(Difference { p_source: Some(f1.clone()), p_target: None });
            } else {
                collector.push_back(Difference { p_source: Some(f1.clone()), p_target: None });
            }
        }
    }

    for f2 in &dir2_set {
        let f1o = dir1_set.get(f2);
        if f1o.is_none() {
            if f2.is_dir() {
                let mrmis = find_differences_rec(None, Some(&f2.path), collector);
                most_recent_modified_in_source = most_recent_modified_in_source.max(mrmis);
                collector.push_front(Difference { p_source: None, p_target: Some(f2.clone()) });
            } else {
                collector.push_back(Difference { p_source: None, p_target: Some(f2.clone()) });
            }
        }
    }

    return most_recent_modified_in_source
}


fn list_paths(dir: Option<&str>) -> HashSet<AnnotatedPath> {
    let mut result = HashSet::new();
    match dir {
        Some(dir) => {
            let reader = fs::read_dir(dir).expect("Could not read from directory");
            for r in reader {
                let e = r.unwrap();
                let path = e.path().to_str().unwrap().to_string();
                let name = e.file_name().to_str().unwrap().to_string();
                let meta = e.metadata().unwrap();
                let is_dir = meta.is_dir();
                let modified = if is_dir {None} else {Some(meta.modified().unwrap())};
                result.insert(AnnotatedPath {path, name, modified});
            }
        }
        None => {}
    }
    return result;
}
