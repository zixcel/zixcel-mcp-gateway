use super::read_with_hook;
use std::{fs, path::PathBuf, process::Command};

fn path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "zixcel-mcp-file-admission-{}-{name}",
        std::process::id()
    ))
}

#[test]
fn exact_limit_is_allowed_and_limit_plus_one_is_rejected() {
    let exact = path("exact");
    let oversized = path("oversized");
    fs::write(&exact, vec![b'a'; 64]).expect("exact fixture");
    fs::write(&oversized, vec![b'b'; 65]).expect("oversized fixture");
    assert_eq!(read_with_hook(&exact, 64, || {}).expect("exact").len(), 64);
    assert!(read_with_hook(&oversized, 64, || {}).is_err());
    fs::remove_file(exact).expect("remove exact fixture");
    fs::remove_file(oversized).expect("remove oversized fixture");
}

#[test]
fn directory_is_not_an_admitted_handle() {
    let directory = path("directory");
    fs::create_dir_all(&directory).expect("directory fixture");
    assert!(read_with_hook(&directory, 64, || {}).is_err());
    fs::remove_dir(directory).expect("remove directory fixture");
}

#[cfg(unix)]
#[test]
fn initial_symbolic_link_is_not_followed() {
    use std::os::unix::fs::symlink;

    let target = path("initial-target");
    let link = path("initial-link");
    fs::write(&target, b"approved").expect("target fixture");
    symlink(&target, &link).expect("link fixture");
    assert!(read_with_hook(&link, 64, || {}).is_err());
    fs::remove_file(link).expect("remove link fixture");
    fs::remove_file(target).expect("remove target fixture");
}

#[cfg(unix)]
#[test]
fn same_size_symlink_swap_does_not_replace_the_open_handle() {
    use std::os::unix::fs::symlink;

    let admitted = path("admitted");
    let held = path("held");
    let replacement = path("replacement");
    fs::write(&admitted, b"approved").expect("admitted fixture");
    fs::write(&replacement, b"replaced").expect("replacement fixture");
    let bytes = read_with_hook(&admitted, 32, || {
        fs::rename(&admitted, &held).expect("hold admitted inode");
        symlink(&replacement, &admitted).expect("replace path with link");
    })
    .expect("open handle remains admitted");
    assert_eq!(bytes, b"approved");
    fs::remove_file(admitted).expect("remove replacement link");
    fs::remove_file(held).expect("remove admitted inode");
    fs::remove_file(replacement).expect("remove replacement fixture");
}

#[cfg(unix)]
#[test]
fn fifo_and_device_are_rejected_without_blocking() {
    let fifo = path("fifo");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo is available")
            .success()
    );
    assert!(read_with_hook(&fifo, 64, || {}).is_err());
    fs::remove_file(fifo).expect("remove fifo");
    assert!(read_with_hook(std::path::Path::new("/dev/null"), 64, || {}).is_err());
}
