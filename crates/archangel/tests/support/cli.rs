//! Shared by the command line tests: run the binary in a fixture repository.
#![cfg(test)]

use std::process::Output;

use archangel_testkit::FixtureRepo;

/// What one run of the binary gave.
pub(crate) struct Finished {
    pub(crate) status: i32,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

impl From<Output> for Finished {
    fn from(output: Output) -> Self {
        Self {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

/// The binary, in the repository's folder, with color and the fail points off.
pub(crate) fn command(repo: &FixtureRepo) -> assert_cmd::Command {
    let mut command = assert_cmd::Command::cargo_bin("archangel").unwrap();
    command
        .current_dir(repo.path())
        .env_remove("FAILPOINTS")
        .env_remove("NO_COLOR");
    command
}

/// Runs the binary with `args` in the repository's folder.
pub(crate) fn run(repo: &FixtureRepo, args: &[&str]) -> Finished {
    command(repo).args(args).output().unwrap().into()
}
