pub(crate) fn print_app() {
  let name = env!("CARGO_CRATE_NAME");
  let version = env!("CARGO_PKG_VERSION");
  eprintln!("{name} {version}");
}
