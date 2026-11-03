//! Test support: small repositories written to a temporary folder.

use std::fs;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, PoisonError};

use tempfile::TempDir;

/// A repository in a temporary folder, removed when the value is dropped.
#[derive(Debug)]
pub struct FixtureRepo {
    dir: TempDir,
}

impl FixtureRepo {
    /// An empty repository.
    ///
    /// # Panics
    ///
    /// When the temporary folder cannot be made.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test support fails the test when the disk does"
    )]
    pub fn new() -> Self {
        Self {
            dir: TempDir::new().expect("a temporary folder"),
        }
    }

    /// Writes a text file at a path relative to the repository, with its folders.
    #[must_use]
    pub fn file(self, path: &str, text: &str) -> Self {
        self.bytes(path, text.as_bytes())
    }

    /// Writes a file from raw bytes.
    ///
    /// # Panics
    ///
    /// When the file cannot be written.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test support fails the test when the disk does"
    )]
    pub fn bytes(self, path: &str, bytes: &[u8]) -> Self {
        let full = self.dir.path().join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("the file's folder");
        }
        fs::write(&full, bytes).expect("the file");
        self
    }

    /// Makes a symbolic link at `path` that points to `target`, as written.
    ///
    /// # Panics
    ///
    /// When the link cannot be made.
    #[cfg(unix)]
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test support fails the test when the disk does"
    )]
    pub fn symlink(self, path: &str, target: &str) -> Self {
        let full = self.dir.path().join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("the link's folder");
        }
        std::os::unix::fs::symlink(target, &full).expect("the link");
        self
    }

    /// Removes every permission from a file or folder, so that the user cannot read it.
    ///
    /// # Panics
    ///
    /// When the permissions cannot be changed.
    #[cfg(unix)]
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test support fails the test when the disk does"
    )]
    pub fn unreadable(self, path: &str) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let full = self.dir.path().join(path);
        fs::set_permissions(&full, fs::Permissions::from_mode(0o000)).expect("the permissions");
        self
    }

    /// Writes `archangel.config.json` from a JSON value.
    #[must_use]
    pub fn json_config(self, config: &serde_json::Value) -> Self {
        let text = serde_json::to_string_pretty(config).unwrap_or_default();
        self.file("archangel.config.json", &text)
    }

    /// Writes the fixture plugin to `fixture-plugin.mjs` and `archangel.config.mjs`, a config
    /// with the fields of `config` that loads the plugin as `fixture`, so a rule can use
    /// `plugin: 'fixture/<condition>'`.
    #[must_use]
    pub fn plugin_config(self, config: &serde_json::Value) -> Self {
        let fields = serde_json::to_string_pretty(config).unwrap_or_default();
        let text = format!(
            "import fixture from './fixture-plugin.mjs';\n\
             export default {{ ...{fields}, plugins: {{ fixture }} }};\n"
        );
        self.file("fixture-plugin.mjs", FIXTURE_PLUGIN)
            .file("archangel.config.mjs", &text)
    }

    /// Adds `count` files `<folder>/f<i>.ts`, where each file imports the `imports` files before it,
    /// so the files form no cycle. File 0 imports nothing.
    #[must_use]
    pub fn synthetic(self, folder: &str, count: usize, imports: usize) -> Self {
        (0..count).fold(self, |repo, i| {
            let text: String = (i.saturating_sub(imports)..i)
                .map(|j| format!("import './f{j}';\n"))
                .chain(std::iter::once(format!("export const f{i} = {i};\n")))
                .collect();
            repo.file(&format!("{folder}/f{i}.ts"), &text)
        })
    }

    /// Writes a file in place, as an editor's save does, creating its folders.
    ///
    /// # Panics
    ///
    /// When the disk refuses.
    #[expect(
        clippy::expect_used,
        reason = "test support fails the test when the disk does"
    )]
    pub fn write(&self, path: &str, text: &str) {
        let full = self.join(path);
        if let Some(folder) = full.parent() {
            fs::create_dir_all(folder).expect("the folder");
        }
        fs::write(full, text).expect("the file");
    }

    /// Deletes a file.
    ///
    /// # Panics
    ///
    /// When the disk refuses.
    #[expect(
        clippy::expect_used,
        reason = "test support fails the test when the disk does"
    )]
    pub fn remove(&self, path: &str) {
        fs::remove_file(self.join(path)).expect("the file is deleted");
    }

    /// Deletes a folder and everything in it.
    ///
    /// # Panics
    ///
    /// When the disk refuses.
    #[expect(
        clippy::expect_used,
        reason = "test support fails the test when the disk does"
    )]
    pub fn remove_folder(&self, path: &str) {
        fs::remove_dir_all(self.join(path)).expect("the folder is deleted");
    }

    /// Moves a file, over another one if it exists.
    ///
    /// # Panics
    ///
    /// When the disk refuses.
    #[expect(
        clippy::expect_used,
        reason = "test support fails the test when the disk does"
    )]
    pub fn rename(&self, from: &str, to: &str) {
        fs::rename(self.join(from), self.join(to)).expect("the file is moved");
    }

    /// The text of a file in the repository, or an empty string when there is none.
    #[must_use]
    pub fn read(&self, path: &str) -> String {
        fs::read_to_string(self.join(path)).unwrap_or_default()
    }

    /// The repository folder.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// A path inside the repository.
    #[must_use]
    pub fn join(&self, path: &str) -> PathBuf {
        self.dir.path().join(path)
    }
}

impl Default for FixtureRepo {
    fn default() -> Self {
        Self::new()
    }
}

/// The fixture plugin: conditions that report imports, count imports or importers, throw, stop
/// their process, report a file outside their view, describe their records, write a line straight
/// to standard output, and wait on a [`Gate`].
pub const FIXTURE_PLUGIN: &str = include_str!("fixture_plugin.mjs");

/// A gate that the fixture plugin's `waits` condition waits on until the test opens it: a
/// listener whose connections stay open until then.
#[derive(Debug)]
pub struct Gate {
    port: u16,
    state: Arc<(Mutex<GateState>, Condvar)>,
}

#[derive(Debug, Default)]
struct GateState {
    open: bool,
    waiting: Vec<TcpStream>,
}

impl Gate {
    /// A closed gate, listening on a free port of this machine.
    ///
    /// # Panics
    ///
    /// When no port is free.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test support fails the test when the machine has no free port"
    )]
    pub fn closed() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
        let port = listener.local_addr().expect("the port").port();
        let state = Arc::new((Mutex::new(GateState::default()), Condvar::new()));
        let held = Arc::clone(&state);
        // The thread waits for connections until the test process ends.
        std::thread::spawn(move || {
            let (lock, arrived) = &*held;
            for stream in listener.incoming().flatten() {
                let mut state = lock.lock().unwrap_or_else(PoisonError::into_inner);
                if state.open {
                    stream.shutdown(Shutdown::Both).ok();
                } else {
                    state.waiting.push(stream);
                    arrived.notify_all();
                }
            }
        });
        Self { port, state }
    }

    /// The port the fixture plugin's `waits` condition connects to.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Opens the gate: every waiting connection closes, and every later one closes at once.
    pub fn open(&self) {
        let mut state = self.state.0.lock().unwrap_or_else(PoisonError::into_inner);
        state.open = true;
        for stream in state.waiting.drain(..) {
            stream.shutdown(Shutdown::Both).ok();
        }
    }

    /// Returns once a condition waits at the closed gate.
    pub fn await_a_waiter(&self) {
        let (lock, arrived) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(PoisonError::into_inner);
        while state.waiting.is_empty() {
            state = arrived.wait(state).unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Returns once every condition waiting at the closed gate has gone, because its process
    /// closed the connection.
    pub fn await_the_waiters_leaving(&self) {
        let waiting: Vec<TcpStream> = {
            let state = self.state.0.lock().unwrap_or_else(PoisonError::into_inner);
            state
                .waiting
                .iter()
                .filter_map(|stream| stream.try_clone().ok())
                .collect()
        };
        for mut stream in waiting {
            std::io::copy(&mut stream, &mut std::io::sink()).ok();
        }
    }
}

/// Runs one test again in a child process with extra environment variables, because setting
/// variables inside a test process needs `unsafe` code and leaks into every other test.
///
/// Returns `true` in the child, where the test's assertions run, and `false` in the parent,
/// after the child passed.
///
/// # Panics
///
/// In the parent, when the child cannot start or its test fails.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "test support fails the test when the child does"
)]
pub fn in_child_with_env(test: &str, env: &[(&str, &std::ffi::OsStr)]) -> bool {
    const MARKER: &str = "ARCHANGEL_TEST_CHILD";
    if std::env::var_os(MARKER).is_some() {
        return true;
    }
    let exe = std::env::current_exe().expect("the test binary");
    let status = std::process::Command::new(exe)
        .args(["--exact", test, "--test-threads=1"])
        .env(MARKER, "1")
        .envs(env.iter().copied())
        .status()
        .expect("the child test process");
    assert!(
        status.success(),
        "child test {test} failed ({status}); rerun it with ARCHANGEL_TEST_CHILD=1 to see why"
    );
    false
}

/// A repository with an entity that imports shared code twice and one that imports nothing, and
/// a config with these rules over the fixture plugin.
#[must_use]
pub fn entities_importing_shared(rules: &serde_json::Value) -> FixtureRepo {
    FixtureRepo::new()
        .file("src/shared/s.ts", "export const s = 1;\n")
        .file("src/shared/t.ts", "export const t = 1;\n")
        .file(
            "src/entities/user/index.ts",
            "import { s } from '../../shared/s';\nimport { t } from '../../shared/t';\n",
        )
        .file("src/entities/post/index.ts", "export const post = 1;\n")
        .plugin_config(&serde_json::json!({
            "elements": { "entity": "src/entities/{entity}/**", "shared": "src/shared/**" },
            "rules": rules,
        }))
}

/// A config with two elements, `app` under `src/app` and `shared` for the rest of `src`, and
/// these rules.
#[must_use]
pub fn app_and_shared(rules: &[serde_json::Value]) -> serde_json::Value {
    serde_json::json!({
        "elements": { "app": "src/app/**", "shared": "src/**" },
        "rules": rules,
    })
}

/// The text of a minimal Yarn Plug'n'Play `.pnp.cjs` for a root workspace with these packages,
/// each given as its name and its folder relative to the repository.
#[must_use]
pub fn pnp_manifest(packages: &[(&str, &str)]) -> String {
    let dependencies: Vec<serde_json::Value> = packages
        .iter()
        .map(|(name, _)| serde_json::json!([name, "npm:1.0.0"]))
        .collect();
    let root = serde_json::json!({ "packageLocation": "./", "packageDependencies": dependencies });
    let mut registry = vec![
        serde_json::json!([null, [[null, root]]]),
        serde_json::json!(["root", [["workspace:.", root]]]),
    ];
    for (name, location) in packages {
        let info = serde_json::json!({
            "packageLocation": location,
            "packageDependencies": [[name, "npm:1.0.0"]],
        });
        registry.push(serde_json::json!([name, [["npm:1.0.0", info]]]));
    }
    let state = serde_json::json!({
        "enableTopLevelFallback": false,
        "ignorePatternData": null,
        "dependencyTreeRoots": [{ "name": "root", "reference": "workspace:." }],
        "fallbackPool": [],
        "fallbackExclusionList": [],
        "packageRegistryData": registry,
    });
    let escaped = state.to_string().replace('\\', "\\\\").replace('\'', "\\'");
    format!("#!/usr/bin/env node\nconst RAW_RUNTIME_STATE =\n'{escaped}';\n")
}

/// An import table as a JSON config writes it: its name, its rows and its options.
#[must_use]
pub fn table(
    name: &str,
    rows: &serde_json::Value,
    options: &serde_json::Value,
) -> serde_json::Value {
    let mut table = options.clone();
    table["kind"] = serde_json::json!("mayImport");
    table["name"] = serde_json::json!(name);
    table["rows"] = rows.clone();
    table
}
