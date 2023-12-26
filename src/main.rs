mod tests;
mod differences;

use std::{env, fs, io};
use std::hash::{Hash, Hasher};
use std::process::exit;
use differences::verify_source_fully_newer_than_target;
use crate::differences::apply_diffs_source_to_target_with_prints;

fn main() {
    let mut args: Vec<String> = env::args().collect();

    args = Vec::from(["path-to-exe".to_string(), "test-env-dirs/stick orig/".to_string(), "test-env-dirs/stick backup/".to_string()]);

    if args.len() != 3 {
        println!("Invalid arguments (received {}, expected 2).", args.len() - 1);
        println!("Excepted argument structure:");
        println!("[\"source-path\", \"backup-path\"]");
        println!("Received argument structure:");
        println!("{:?}", &args[1..]);
        println!("Try again. Exiting...");
        exit(0);
    }

    let source_path = args[1].to_string();
    println!("Source Path: \"{}\"", source_path);
    if !fs::metadata(&source_path).is_ok_and(|m| m.is_dir()) {
        println!("Source Path is not a directory. Exiting...");
        exit(0);
    }

    let target_path = args[2].to_string();
    println!("Backup Path: \"{}\"", target_path);
    if !fs::metadata(&target_path).is_ok_and(|m| m.is_dir()) {
        println!("Backup Path is not a directory. Exiting...");
        exit(0);
    }

    if source_path == target_path {
        println!("Source Path is the same as Backup Path. Exiting...");
        exit(0);
    }

    println!("Will now analyse directories and verify that backup directory does not contain any files that\n    \
              are newer than their expression in the source and\n    \
              that backup directory does not contain any files that don't exist in source,\n    \
              but are newer than the last common modification date (assumed time of last synchronization).");

    let (most_recent_modified_in_source, diffs) = differences::find_differences(&source_path, &target_path);
    if diffs.is_empty() {
        println!("Found NO differences. Backup is up-to-date.");
        exit(0);
    }
    println!("Differences:");
    for d in &diffs {
        println!("{:?}", d);
    }

    let problems = verify_source_fully_newer_than_target(most_recent_modified_in_source, &diffs);
    if !problems.is_empty() {
        println!("Problems:");
        for p in &problems {
            println!("{:?}", p.1);
        }
    }

    if !&problems.is_empty() {
        println!("Problems found (see above).\n    \
            Please study the problems carefully and decide how to proceed.
            To simply override ALL changes in the backup directory,\n    \
            please type \"continue\".    \
            If you type anything else, the program will exit.");
    } else {
        println!("{} differences found (see above).\n    \
            0 Problems were detected, but there is no guarantee that this is correct.\n    \
            Please study the differences in detail and choose whether you want to continue.\n    \
            To proceed please type \"continue\".    \
            If you type anything else, the program will exit.", diffs.len());
    }
    let mut s = String::new();
    io::stdin().read_line(&mut s).expect("stdio error");
    match s.trim() {
        "continue" => {},
        _ => {
            println!("Ok. Exiting...");
            exit(0)
        }
    }

    println!("Found {} differences. Overriding all in backup directory.", &diffs.len());

    //directories need to be done first...
    apply_diffs_source_to_target_with_prints(&source_path, &target_path, diffs.iter());
}
