#[cfg(test)]
mod tests {
  use std::{
    io::Write,
    path::{MAIN_SEPARATOR_STR, PathBuf},
  };

  use color_eyre::eyre::eyre;
  use oxc_allocator::Allocator;
  use oxc_ast_visit::Visit;
  use oxc_parser::Parser;
  use oxc_span::SourceType;
  use pretty_assertions::assert_eq;
  use tempdir::TempDir;

  use crate::{Config, Entry, visitor::I18NVisitor};

  fn parse(path: &PathBuf, config: Config) -> Vec<Entry> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).expect("should determine source type");
    let source_text = std::fs::read_to_string(path).expect("should read file");
    let ret = Parser::new(&allocator, source_text.as_str(), source_type).parse();
    log::debug!("Program: {:#?}", ret.program.body);

    let program = ret.program;

    let mut visitor = I18NVisitor::new(&allocator, &program, path, &config);
    visitor.visit_program(&program);
    visitor.entries
  }

  fn get_config<T: Into<PathBuf> + Clone>(path: T) -> color_eyre::Result<Config> {
    let mut config = Config::new(path.clone(), false, false)?;
    config.locales = vec!["en".into(), "fr".into()];
    config.output = ["locales", "$LOCALE", "$NAMESPACE.json"].join(MAIN_SEPARATOR_STR);
    config.input = vec!["**/*.{ts,tsx}".into()];
    Ok(config)
  }

  fn write_file(path: &PathBuf, content: &str) -> color_eyre::Result<()> {
    let parent = path.parent().ok_or(eyre!("Cannot find parent"))?;
    std::fs::create_dir_all(parent)?;
    let mut file = std::fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
  }

  fn write_ts_config(dir: &TempDir) {
    let tsconfig = dir.path().join("tsconfig.json");
    write_file(
      &tsconfig,
      r#"
{
  "compilerOptions": {
    "root": "src",
    "baseUrl": "src",
    /* Base Options: */
    "esModuleInterop": true,
    "skipLibCheck": true,
    "target": "es2022",
    "allowJs": true,
    "resolveJsonModule": true,
    "moduleDetection": "force",
    "isolatedModules": true,
    "verbatimModuleSyntax": true,
    /* Strictness */
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    /* If NOT transpiling with TypeScript: */
    "module": "preserve",
    "noEmit": true,
    /* If your code doesn't run in the DOM: */
    "lib": ["es2022"],
    "forceConsistentCasingInFileNames": true,
    "skipLibCheck": true,
    "paths": {
      "~/*": ["./src/*"],
    }
  }
}
        "#,
    )
    .expect("should write file");
  }

  #[test_log::test]
  fn resolve_string_type_import_constraint_from_import() {
    let dir = TempDir::new("translations").expect("should create tempdir");
    let main = dir.path().join("src").join("main.tsx");
    write_file(
      &main,
      r#"import { Role } from './utils';
export function InvitationEmail() {
  const role: Role;
  return (
    <EmailRoot>
      <Text className='truncate'>
        <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
      </Text>
    </EmailRoot>
  );
}"#,
    )
    .expect("should write file");

    write_ts_config(&dir);

    let utils = dir.path().join("src").join("utils.ts");
    write_file(&utils, r#"export type Role = 'admin' | 'member' | 'owner'"#).expect("should write file");
    let config = get_config(dir.path()).expect("should get config");
    let entries = parse(&main, config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries, vec![Entry::new_with_context(
      "role",
      "Role",
      "ns",
      vec!("admin".to_string(), "member".to_string(), "owner".to_string())
    )]);
  }

  #[test_log::test]
  fn resolve_string_type_from_path_import() {
    let dir = TempDir::new("translations").expect("should create tempdir");
    let main = dir.path().join("src").join("main.tsx");
    write_file(
      &main,
      r#"import { Role } from '~/utils';
export function InvitationEmail() {
  const role: Role;
  return (
    <EmailRoot>
      <Text className='truncate'>
        <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
      </Text>
    </EmailRoot>
  );
}"#,
    )
    .expect("should write file");

    write_ts_config(&dir);

    let utils = dir.path().join("src").join("utils.ts");
    write_file(&utils, r#"export type Role = 'admin' | 'member' | 'owner'"#).expect("should write file");
    let config = get_config(dir.path()).expect("should get config");
    let entries = parse(&main, config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries, vec![Entry::new_with_context(
      "role",
      "Role",
      "ns",
      vec!("admin".to_string(), "member".to_string(), "owner".to_string())
    )]);
  }
  #[test_log::test]
  fn resolve_string_type_from_import() {
    let dir = TempDir::new("translations").expect("should create tempdir");
    let main = dir.path().join("src").join("main.tsx");
    write_file(
      &main,
      r#"import { Role } from './utils';
export function InvitationEmail() {
  const role: Role;
  return (
    <EmailRoot>
      <Text className='truncate'>
        <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
      </Text>
    </EmailRoot>
  );
}"#,
    )
    .expect("should write file");

    write_ts_config(&dir);

    let utils = dir.path().join("src").join("utils.ts");
    write_file(&utils, r#"export type Role = 'admin' | 'member' | 'owner'"#).expect("should write file");
    let config = get_config(dir.path()).expect("should get config");
    let entries = parse(&main, config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries, vec![Entry::new_with_context(
      "role",
      "Role",
      "ns",
      vec!("admin".to_string(), "member".to_string(), "owner".to_string())
    )]);
  }

  #[test_log::test]
  fn resolve_function_return_type_from_import() {
    let dir = TempDir::new("translations").expect("should create tempdir");
    let main = dir.path().join("src").join("main.tsx");
    write_file(
      &main,
      r#"import { getRole } from './utils';
export function InvitationEmail() {
  const role = getRole();
  return (
    <EmailRoot>
      <Text className='truncate'>
        <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
      </Text>
    </EmailRoot>
  );
}"#,
    )
    .expect("should write file");

    write_ts_config(&dir);

    let utils = dir.path().join("src").join("utils.ts");
    write_file(&utils, r#"export function getRole(): 'admin' | 'member' | 'owner' { return 'admin'; }"#)
      .expect("should write file");
    let config = get_config(dir.path()).expect("should get config");
    let entries = parse(&main, config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries, vec![Entry::new_with_context(
      "role",
      "Role",
      "ns",
      vec!("admin".to_string(), "member".to_string(), "owner".to_string())
    )]);
  }

  #[test_log::test]
  fn resolve_function_return_type_from_import_with_another_type() {
    let dir = TempDir::new("translations").expect("should create tempdir");
    let main = dir.path().join("src").join("main.tsx");
    write_file(
      &main,
      r#"import { getRole } from './utils';
export function InvitationEmail() {
  const role = getRole();
  return (
    <EmailRoot>
      <Text className='truncate'>
        <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
      </Text>
    </EmailRoot>
  );
}"#,
    )
    .expect("should write file");

    write_ts_config(&dir);

    let utils = dir.path().join("src").join("utils.ts");
    write_file(
      &utils,
      r#"type Members = 'admin' | 'member' | 'owner';
        export function getRole(): Members { return 'admin'; }"#,
    )
    .expect("should write file");
    let config = get_config(dir.path()).expect("should get config");
    let entries = parse(&main, config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries, vec![Entry::new_with_context(
      "role",
      "Role",
      "ns",
      vec!("admin".to_string(), "member".to_string(), "owner".to_string())
    )]);
  }

  #[test_log::test]
  fn resolve_function_return_type_from_import_with_another_type_from_as_const() {
    let dir = TempDir::new("translations").expect("should create tempdir");
    let main = dir.path().join("src").join("main.tsx");
    write_file(
      &main,
      r#"import type { Roles } from './utils';
export function InvitationEmail({ role }: { role: Roles }) {
  return (
    <EmailRoot>
      <Text className='truncate'>
        <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
      </Text>
    </EmailRoot>
  );
}"#,
    )
    .expect("should write file");

    write_ts_config(&dir);

    let utils = dir.path().join("src").join("utils.ts");
    write_file(
      &utils,
      r#"
      const roles = ['admin', 'member', 'owner'] as const;
      type Roles = (typeof roles)[number];
      export function getRole(): Roles { return 'admin'; }
      "#,
    )
    .expect("should write file");
    let config = get_config(dir.path()).expect("should get config");
    let entries = parse(&main, config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries, vec![Entry::new_with_context(
      "role",
      "Role",
      "ns",
      vec!("admin".to_string(), "member".to_string(), "owner".to_string())
    )]);
  }

  #[test_log::test]
  fn resolve_type_from_double_import() {
    let dir = TempDir::new("translations").expect("should create tempdir");
    let main = dir.path().join("src").join("main.tsx");
    write_file(
      &main,
      r#"import type { Roles } from './utils';
export function InvitationEmail({ role }: { role: Roles }) {
  return (
    <EmailRoot>
      <Text className='truncate'>
        <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
      </Text>
    </EmailRoot>
  );
}"#,
    )
    .expect("should write file");

    write_ts_config(&dir);

    let utils = dir.path().join("src").join("utils.ts");
    write_file(
      &utils,
      r#"
      import type { roles } from './roles';
      type Roles = (typeof roles)[number];
      export function getRole(): Roles { return 'admin'; }
      "#,
    )
    .expect("should write file");

    let roles = dir.path().join("src").join("roles.ts");
    write_file(
      &roles,
      r#"
        export const roles = ['admin', 'member', 'owner'] as const;
      "#,
    )
    .expect("should write file");

    let config = get_config(dir.path()).expect("should get config");
    let entries = parse(&main, config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries, vec![Entry::new_with_context(
      "role",
      "Role",
      "ns",
      vec!("admin".to_string(), "member".to_string(), "owner".to_string())
    )]);
  }
}
