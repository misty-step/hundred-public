use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=HUNDRED_BUILD_SHA");
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    let mut references = vec!["HEAD".to_owned(), "packed-refs".to_owned()];
    if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"]) {
        references.push(reference);
    }
    for reference in references {
        if let Some(path) = git(&["rev-parse", "--git-path", &reference])
            && std::path::Path::new(&path).exists()
        {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    let revision = std::env::var("HUNDRED_BUILD_SHA")
        .ok()
        .or_else(|| git(&["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "source-archive".into());
    println!("cargo:rustc-env=HUNDRED_REVISION={revision}");
}
