use std::path::PathBuf;

use color_eyre::owo_colors::OwoColorize;
use oxc_resolver::{AliasValue, ResolveOptions, Resolver, TsConfig, TsconfigOptions, TsconfigReferences};

pub trait ResolveFromTsConfig {
  fn from_ts_config<P>(path: P) -> Option<Self>
  where
    Self: Sized,
    P: Into<PathBuf>;
}

fn find_tsconfig<P>(path: P) -> Option<PathBuf>
where
  P: Into<PathBuf>,
{
  let path = path.into();
  if path.ends_with("tsconfig.json") {
    Some(path)
  } else if path.join("tsconfig.json").exists() {
    Some(path.join("tsconfig.json"))
  } else {
    let parent = path.parent()?;
    if parent.join("tsconfig.json").exists() {
      Some(parent.join("tsconfig.json"))
    } else {
      find_tsconfig(parent)
    }
  }
}

impl ResolveFromTsConfig for Resolver {
  fn from_ts_config<P>(path: P) -> Option<Self>
  where
    Self: Sized,
    P: Into<PathBuf>,
  {
    let working_dir = path.into();
    let working_dir = if working_dir.ends_with("tsconfig.json") {
      working_dir.parent().expect("the parent must be defined")
    } else {
      &working_dir
    };

    let tsconfig = find_tsconfig(working_dir)?;
    let config = if let Ok(mut tsconfig_content) = std::fs::read_to_string(&tsconfig) {
      TsConfig::parse(true, working_dir, tsconfig_content.as_mut_str()).ok()
    } else {
      None
    };

    let config = config.as_ref();

    let alias = config
      .and_then(|config| {
        config.compiler_options.paths.as_ref().map(|paths| {
          paths
            .iter()
            .map(|(key, value)| {
              (
                key.clone(),
                value
                  .iter()
                  .map(|val| AliasValue::Path(working_dir.join(val).to_string_lossy().to_string()))
                  .collect::<Vec<_>>(),
              )
            })
            .collect::<Vec<_>>()
        })
      })
      .inspect(|alias| {
        if alias.is_empty() {
          log::warn!("No alias found in tsconfig.json");
        }
        log::trace!("Alias found in tsconfig.json: {:?}", alias.bright_black().italic());
      })
      .unwrap_or_default();

    let roots = config
      .and_then(|config| config.compiler_options.base_url.as_ref())
      .map(|roots| roots.iter().map(|root| working_dir.join(root)).collect::<Vec<_>>())
      .unwrap_or_default();

    let options = ResolveOptions {
      roots,
      alias,
      extensions: vec![".d.ts".into(), ".ts".into(), ".tsx".into(), ".js".into(), ".jsx".into()],
      extension_alias: vec![
        (".js".into(), vec![".js".into(), ".ts".into(), ".d.ts".into()]),
        (".jsx".into(), vec![".jsx".into(), ".tsx".into()]),
      ],
      condition_names: vec!["node".into(), "types".into(), "import".into()],
      prefer_relative: true,
      tsconfig: {
        if tsconfig.exists() {
          let options = TsconfigOptions {
            config_file: tsconfig,
            references: TsconfigReferences::Auto,
          };
          Some(options)
        } else {
          None
        }
      },
      ..Default::default()
    };

    log::trace!("Instanciating resolver");
    Some(Resolver::new(options))
  }
}
