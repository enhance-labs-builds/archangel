// Which platform package holds the binary for this machine. Kept apart from the launcher so the
// tests can ask about any platform.

/** The platform packages, by the key the launcher computes for a machine. */
export const PACKAGES = {
  'darwin-arm64': '@enhance-eng/archangel-darwin-arm64',
  'darwin-x64': '@enhance-eng/archangel-darwin-x64',
  'linux-arm64-gnu': '@enhance-eng/archangel-linux-arm64-gnu',
  'linux-arm64-musl': '@enhance-eng/archangel-linux-arm64-musl',
  'linux-x64-gnu': '@enhance-eng/archangel-linux-x64-gnu',
  'linux-x64-musl': '@enhance-eng/archangel-linux-x64-musl',
  'win32-x64': '@enhance-eng/archangel-win32-x64',
};

/** The key of a machine: the platform and the CPU, and on Linux the C library. */
export function platformKey({ platform, arch, musl }) {
  if (platform !== 'linux') return `${platform}-${arch}`;
  return `${platform}-${arch}-${musl ? 'musl' : 'gnu'}`;
}

/** Whether this Linux uses musl: Node reports a glibc version only when it runs on glibc. */
export function usesMusl(report = process.report?.getReport()) {
  return process.platform === 'linux' && !report?.header?.glibcVersionRuntime;
}

/** What to say when no binary was built for the machine. */
export function unsupported(key) {
  return (
    `archangel has no prebuilt binary for ${key}. ` +
    'Install it from source with `cargo install archangel`, which needs Rust.'
  );
}

/** What to say when the machine's package is not installed, as after `npm install --no-optional`. */
export function missing(key, name) {
  return (
    `archangel's binary for ${key} is in the package ${name}, which is not installed. ` +
    'Install without --no-optional or --omit=optional, or install it from source with ' +
    '`cargo install archangel`.'
  );
}
