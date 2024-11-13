use std::{
  collections::HashMap,
  io::Write,
  path::{Path, PathBuf, MAIN_SEPARATOR_STR},
};

use clap::Parser;
use color_eyre::{eyre::eyre, owo_colors::OwoColorize};
use flatten_json_object::Flattener;
use i18next_parser::{
  cli::{Cli, Runnable},
  utils::initialize_logging,
};
use i18next_parser_core::{merge_all_values, parse_directory, Config, IsEmpty, MergeResults};
use log::debug;
use pretty_assertions::assert_eq;
use serde::Serialize;
use serde_json::{json, Value};
use tempdir::TempDir;

fn setup_test<T: Into<PathBuf> + Clone>(path: T) -> color_eyre::Result<(T, Config)> {
  let _ = initialize_logging(&false);

  let mut config = Config::new(path.clone(), false)?;
  config.locales = vec!["en".into(), "fr".into()];
  config.output = ["locales", "$LOCALE", "$NAMESPACE.json"].join(MAIN_SEPARATOR_STR);
  config.input = vec!["**/*.{ts,tsx}".into()];

  Ok((path, config))
}

#[test]
fn should_parse_successfully() {
  let dir = TempDir::new("translations").unwrap();
  {
    const SOURCES: &str = r#" const ns = "reset-password" satisfies Ns;export const action: ActionFunction = async ({ request }) => {const locale = await i18next.getLocale(request);const t = await i18next.getFixedT(locale, ns);const title = t("toast.title", "Reset password");const tt = t("toast.title", { defaultValue: "Reset password", namespace: "ns" });const { response, data } = await getValidatedFormDataWithResponse(request, schema, {title, text: t("toast.validation.error", "There is an error in the form"), iconType: "password", variant: "destructive",},);if (response) return response;return await handleRemoteQuery(async () => {await resetPassword(data);return redirectWithToast($path("/login"), {title, text: t("toast.text.success", "An email has been sent",), iconType: "password", variant: "default",});}, {title, text: t("toast.text.error", "An error has occurred while trying to reset the password.",), variant: "destructive", iconType: "password",},);};export default function ResetPasswordDialog() {const navigate = useNavigate();const state = useGlobalSubmittingState();return (<Dialog defaultOpen onOpenChange={(isOpen) => if (!isOpen) navigate($path("/login"));}><DialogContent><DialogHeader><DialogTitle><Trans ns={ns} i18nKey="dialog.title">Reset password</Trans></DialogTitle><DialogDescription><Trans ns={ns} i18nKey="dialog.description">Enter your email address in the form below and we will send<br />you further instructions on how to reset your password.</Trans></DialogDescription></DialogHeader><Form method="POST" schema={schema} defaultValue={{ email: "" }}><Email /><DialogFooter><DialogClose asChild><Button variant="outline" isLoading={state !== "idle"} icon={<ApplicationIcon icon="close" />} type="reset"><Trans ns={ns} i18nKey="button.clear">Clear</Trans></Button></DialogClose><Button type="submit" isLoading={state !== "idle"} icon={<ApplicationIcon icon="send" />}><Trans ns={ns} i18nKey="button.submit">Reset password</Trans></Button></DialogFooter></Form></DialogContent></Dialog>);} "#;
    let path = dir.path().join("src").join("main.tsx");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(SOURCES.as_bytes()).unwrap();
  }

  let (working_path, config) = &setup_test(dir.path()).unwrap();

  let path = PathBuf::from(working_path);

  assert!(path.exists(), "{} doesn't exists", path.display().yellow());
  let entries = parse_directory(path, config).unwrap();

  let entries = merge_all_values(entries, config).unwrap();
  for entry in entries {
    let MergeResults {
      namespace: _namespace,
      locale: _locale,
      path: _path,
      backup: _backup,
      merged,
      old_catalog: _old_catalog,
    } = entry;

    assert_eq!(merged.old_count, 0, "there isn't any values yet");
    assert_eq!(merged.merge_count, 0, "there is 0 values to merge");
    assert_eq!(merged.pull_count, 0, "there is 8 new values");
    assert_eq!(merged.reset_count, 0, "there is 0 values to reset");
    assert!(merged.old.is_empty(), "there isn't any old values");
    assert!(merged.reset.is_empty(), "there isn't any reset values");
    assert!(!merged.new.is_empty(), "values must be parsed");
    let new = Flattener::new().set_key_separator(".").flatten(&merged.new).unwrap();
    let new = new.as_object().unwrap();
    println!("New: {:?}", new);
    for key in [
      "toast.title",
      "toast.validation.error",
      "toast.text.success",
      "toast.text.error",
      "dialog.title",
      "dialog.description",
      "button.clear",
      "button.submit",
    ] {
      assert!(new.contains_key(key), "the key {key:?} is present in the new catalog");
    }
  }
  drop(dir);
}

fn create_file<P: AsRef<Path>, V: ?Sized + Serialize>(path: P, value: &V) -> color_eyre::Result<()> {
  let path = path.as_ref();
  let parent = path.parent().ok_or(eyre!("unable to get parent of {}", path.display().yellow()))?;
  std::fs::create_dir_all(parent)?;
  let file = std::fs::File::create(path)?;
  serde_json::to_writer_pretty(file, value)?;
  debug!("{} written", path.display().yellow());

  Ok(())
}

#[test]
fn should_not_override_current_values() {
  let _ = initialize_logging(&false);
  let dir = TempDir::new("translations").unwrap();
  let mut map = HashMap::new();

  let raw_en = json!({ "dialog": { "title": "Reset" }});
  map.insert("en", raw_en);
  map.insert("fr", json!({ "dialog": { "title": "[fr] Reset" }}));
  fn create_files(dir: &TempDir, map: &HashMap<&str, Value>) -> color_eyre::Result<()> {
    let source_text = r#"const el = <Trans ns="ns" i18nKey="dialog.title">Reset password</Trans>;"#;
    let dir_path = dir.path();
    let file_path = dir_path.join("src/main.tsx");
    std::fs::create_dir_all(file_path.parent().unwrap())?;
    let mut file = std::fs::File::create(&file_path)?;
    file.write_all(source_text.as_bytes())?;
    debug!("{} written", file_path.display().yellow());

    let locales: Vec<String> = vec!["en".into(), "fr".into()];
    for lang in &locales {
      let file = dir_path.join("locales").join(lang).join("ns.json");
      let raw_val = map.get(lang.as_str()).ok_or(eyre!("Unable to get {} value", lang.yellow()))?;
      create_file(file, raw_val)?;
    }

    let fr = dir_path.join("locales/fr/ns.json");
    let raw_fr = json!({ "dialog": { "title": "[fr] Reset" }});
    create_file(fr, &raw_fr)?;

    let config = Config {
      locales,
      output: ["locales", "$LOCALE", "$NAMESPACE.json"].join(MAIN_SEPARATOR_STR),
      input: vec!["**/*.{ts,tsx}".into()],
      ..Default::default()
    };

    let config_file = dir_path.join(".i18next-parser.json");
    create_file(config_file, &config)?;
    Ok(())
  }

  create_files(&dir, &map).unwrap();
  let args = ["", "-v", dir.path().to_str().unwrap()];
  let cli = Cli::parse_from(args);
  cli.run().unwrap();

  let en: Value = {
    let en = dir.path().join("locales/en/ns.json");
    let en = std::fs::read(en).unwrap();
    serde_json::from_slice(&en).unwrap()
  };
  assert_eq!(en, json!({ "dialog": { "title": "Reset password" } }));

  let fr: Value = {
    let fr = dir.path().join("locales/fr/ns.json");
    let fr = std::fs::read(fr).unwrap();
    serde_json::from_slice(&fr).unwrap()
  };
  let raw_fr = map.get("fr").ok_or(eyre!("Unable to get fr value")).unwrap();
  assert_eq!(fr, *raw_fr);

  drop(dir);
}
