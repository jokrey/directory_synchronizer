mod tests;
mod differences;
mod ui;

use std::{env, fs, io};
use std::process::exit;
use differences::verify_source_fully_newer_than_target;
use crate::differences::{apply_diffs_source_to_target_with_prints, apply_during_analysis_with_prints};
use crate::ui::start_synchronization_ui;

fn main() {
    let args: Vec<String> = env::args().collect();

    //let args = Vec::from(["path-to-exe".to_string(), "test-env-dirs/source".to_string(), "test-env-dirs/target".to_string(), "cmd".to_string()]);
    //let args = Vec::from(["path-to-exe".to_string(), "test-env-dirs/source".to_string(), "test-env-dirs/target".to_string(), "ui".to_string()]);
    //let args = Vec::from(["path-to-exe".to_string(), "test-env-dirs/source".to_string(), "test-env-dirs/target".to_string(), "just-do-it".to_string()]);

    if args.len() == 4 && &args[1] != &args[2] && fs::metadata(&args[1]).is_ok_and(|m| m.is_dir()) && fs::metadata(&args[2]).is_ok_and(|m| m.is_dir()) {
        println!("Source Path: \"{}\"", args[1]);
        println!("Target Path: \"{}\"", args[2]);
        match args[3].as_str() {
            "ui" => {
                start_synchronization_ui(args[1].to_string(), args[2].to_string()).expect("cannot fix ui failed so sad");
                return
            }
            "cmd" => {
                analyze_and_synchronize_with_dialogue(&args[1], &args[2]);
                return
            }
            "just-do-it" => {
                apply_during_analysis_with_prints(&args[1], &args[2]);
                return
            }
            &_ => {}
        }
    }

    println!("Invalid arguments (received {}, expected 2).", args.len() - 1);
    println!("Excepted argument structure:");
    println!("[\"DIR[source-path]\", \"DIR[backup-path]\"] ui/cmd/just-do-it");
    println!("Received argument structure:");
    println!("{:?}", &args[1..]);
    println!("\n::HELP::");
    println!("ui: Will start a UI where each differences to be applies can be selected");
    println!("ui: Will start a command line where each differences and problem is shown and a decision can be made to apply or not");
    println!("just-do-it: Will synchronize the backup directory to the current state of the source directory");
    println!("Program will NEVER change ANY file in source directory (\"{}\")", args[1]);
    println!("Try again. Exiting...");
}

fn analyze_and_synchronize_with_dialogue(source_path: &String, target_path: &String) {
    println!("Will now analyse directories and verify that backup directory does not contain any files that\n    \
              are newer than their expression in the source and\n    \
              that backup directory does not contain any files that don't exist in source,\n    \
              but are newer than the last common modification date (assumed time of last synchronization).");

    let diffs = differences::find_differences(&source_path, &target_path);
    if diffs.is_empty() {
        println!("Found NO differences. Backup is up-to-date.");
        exit(0);
    }
    let problems = verify_source_fully_newer_than_target(&diffs);
    println!("Differences:");
    for d in &diffs {
        println!("{}", d.describe());
        println!("\n    in directory: {}", d.get_directory_path(source_path.len(), target_path.len()));
        match problems.get(d) {
            None => {}
            Some(desc) => {
                println!("\n    Problem: {desc}");
            }
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

    apply_diffs_source_to_target_with_prints(&source_path, &target_path, diffs.iter());
}
