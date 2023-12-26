use std::fs;
use differences::find_differences;
use rand::random;
use crate::differences;
use crate::differences::{apply_diffs_source_to_target_with_prints, verify_source_fully_newer_than_target};

#[test]
fn test_new_file_in_source() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::write(format!("{source_path}/d2/d2d1/d2d1f1"), [5,4,3,2,1]).ok();

    run_synchronization_as_test(&source_path, &target_path, true);
}
#[test]
fn test_file_updated_in_source() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::write(format!("{source_path}/f1"), [5,4,3,2,1]).ok();

    run_synchronization_as_test(&source_path, &target_path, true);
}
#[test]
fn test_file_updated_in_target() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::write(format!("{target_path}/f2"), [5,4,3,2,1]).ok();

    run_synchronization_as_test(&source_path, &target_path, false);
}
#[test]
fn test_stuff_updated_in_source_and_target() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::write(format!("{source_path}/f1"), [5,4,3,2,1]).ok();
    fs::write(format!("{target_path}/f2"), [5,4,3,2,1]).ok();

    fs::write(format!("{source_path}/d3/d3d1/d3d1d1/d3d1d1f1"), [5,4,3,2,1]).ok();
    fs::write(format!("{target_path}/d3/d3d1/d3d1d1/d3d1d1d1/d3d1d1d1f1"), [5,4,3,2,1]).ok();
    fs::write(format!("{target_path}/d3/d3d1/d3d1d1/d3d1d1f2"), [5,4,3,2,1]).ok();
    fs::remove_dir_all(format!("{source_path}/d1")).ok();
    fs::remove_dir_all(format!("{target_path}/d2")).ok();

    run_synchronization_as_test(&source_path, &target_path, false);
}

#[test]
fn test_new_empty_directory_in_source() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::create_dir(format!("{source_path}/d2/d2d2")).ok();

    run_synchronization_as_test(&source_path, &target_path, true);
}

#[test]
fn test_new_full_directory_in_source() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::create_dir(format!("{source_path}/d2/d2d2")).ok();
    fs::write(format!("{source_path}/d2/d2d2/d2d2f1"), [1,7,4,32,2,1]).ok();
    fs::write(format!("{source_path}/d2/d2d2/d2d2f2"), [1,7,4,32,2,1]).ok();
    fs::create_dir(format!("{source_path}/d2/d2d2/d2d2d1")).ok();
    fs::write(format!("{source_path}/d2/d2d2/d2d2d1/d2d2d1f1"), [1,7,4,32,2,1]).ok();

    run_synchronization_as_test(&source_path, &target_path, true);
}

#[test]
fn test_full_directory_deleted_in_target() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::remove_dir_all(format!("{target_path}/d3/")).ok();

    run_synchronization_as_test(&source_path, &target_path, true);
}

#[test]
fn test_new_file_in_target() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::write(format!("{target_path}/f3"), [5,4,3,2,1]).ok();

    run_synchronization_as_test(&source_path, &target_path, false);
}

#[test]
fn test_new_empty_directory_in_target() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::create_dir(format!("{target_path}/d4")).ok();

    run_synchronization_as_test(&source_path, &target_path, false);
}

#[test]
fn test_new_full_directory_in_target() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::create_dir(format!("{target_path}/d4")).ok();
    fs::write(format!("{target_path}/d4/d4f1"), [7,8,9]).ok();
    fs::write(format!("{target_path}/d4/d4f2"), [7,8,9]).ok();

    run_synchronization_as_test(&source_path, &target_path, false);
}

#[test]
fn test_file_deleted_in_source_and_other_file_changed() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::remove_file(format!("{source_path}/f1")).ok();
    fs::write     (format!("{source_path}/d1/d1f2"), [1]).ok();

    //NOTE: no wrongful problem detection
    run_synchronization_as_test(&source_path, &target_path, true);
}



//Wrongly detected problems::
#[test]
fn test_file_deleted_in_source() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::remove_file(format!("{source_path}/f1")).ok();

    //technically not a "problem", but undetectable and if in doubt: problem
    //if there is even one file modified, this does not get wrongly detected
    run_synchronization_as_test(&source_path, &target_path, false);
}

#[test]
fn test_empty_directory_deleted_in_source() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::remove_dir(format!("{source_path}/d3/d3d1/d3d1d1/d3d1d1d1")).ok();

    //technically not a "problem", but undetectable(no possible way, except rely on dir modification date) and if in doubt: problem
    run_synchronization_as_test(&source_path, &target_path, false);
}

#[test]
fn test_full_directory_deleted_in_source() {
    let (source_path, target_path) = generate_clean_test_directory("test-env-dirs");

    fs::remove_dir_all(format!("{source_path}/d3/")).ok();

    //technically not a "problem", but undetectable(would have to determine directory modification date with search) and if in doubt: problem
    run_synchronization_as_test(&source_path, &target_path, false);
}




fn generate_clean_test_directory(path: &str) -> (String, String) {
    let rand =  random::<u64>();
    let source_path = format!("{path}/source_{rand}/");
    let target_path = format!("{path}/target_{rand}/");
    
    fs::remove_dir_all(&source_path).ok();
    fs::remove_dir_all(&target_path).ok();

    fs::create_dir(format!("{source_path}")).ok();
    fs::create_dir(format!("{source_path}/d1")).ok();
    fs::write     (format!("{source_path}/d1/d1f1"), [1,2,3,4,5]).ok();
    fs::write     (format!("{source_path}/d1/d1f2"), [1,2,3,4,5]).ok();
    fs::create_dir(format!("{source_path}/d2")).ok();
    fs::create_dir(format!("{source_path}/d2/d2d1")).ok();
    fs::write     (format!("{source_path}/d2/d2f1"), [1,2,3,4,5]).ok();
    fs::write     (format!("{source_path}/d2/d2f2"), [1,2,3,4,5]).ok();
    fs::write     (format!("{source_path}/d2/d2f3"), [1,2,3,4,5]).ok();
    fs::write     (format!("{source_path}/d2/d2f4"), [1,2,3,4,5]).ok();
    fs::create_dir(format!("{source_path}/d3")).ok();
    fs::create_dir(format!("{source_path}/d3/d3d1")).ok();
    fs::create_dir(format!("{source_path}/d3/d3d1/d3d1d1")).ok();
    fs::create_dir(format!("{source_path}/d3/d3d1/d3d1d1/d3d1d1d1")).ok();
    fs::write     (format!("{source_path}/d3/d3d1/d3d1d1/d3d1d1f1"), [1,2,3,4,5]).ok();
    fs::write     (format!("{source_path}/d3/d3d1/d3d1d1/d3d1d1f2"), [1,2,3,4,5]).ok();
    fs::write     (format!("{source_path}/f1"), [1,2,3,4,5]).ok();
    fs::write     (format!("{source_path}/f2"), [1,2,3,4,5]).ok();

    fs::create_dir(format!("{target_path}")).ok();

    run_synchronization_as_test(&source_path, &target_path, true);

    return (source_path, target_path);
}

fn run_synchronization_as_test(source_path: &str, target_path: &str, problems_assumed_empty: bool) {
    let diffs = find_differences(source_path, target_path);
    println!("diffs: {:?}", diffs);
    let problems = verify_source_fully_newer_than_target(&diffs);
    println!("problems: {:?}", problems);
    assert_eq!(problems_assumed_empty, problems.is_empty());
    apply_diffs_source_to_target_with_prints(source_path, target_path, diffs.iter());

    let diffs = find_differences(source_path, target_path);
    assert!(diffs.is_empty());
}