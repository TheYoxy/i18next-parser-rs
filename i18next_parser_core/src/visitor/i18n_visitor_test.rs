use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::{Config, Entry, visitor::I18NVisitor};

fn parse(source_text: &str) -> Vec<Entry> {
  let allocator = Allocator::default();
  let source_type = SourceType::from_path("file.tsx").unwrap();
  let ret = Parser::new(&allocator, source_text, source_type).parse();
  log::trace!("Program: {:#?}", ret.program.body);

  let program = ret.program;

  let config = Config::default();
  let mut visitor = I18NVisitor::new(&allocator, &program, "file.tsx", &config);
  visitor.visit_program(&program);
  visitor.entries
}

fn parse_with_options(source_text: &str) -> Vec<Entry> {
  let allocator = Allocator::default();
  let source_type = SourceType::from_path("file.tsx").unwrap();
  let ret = Parser::new(&allocator, source_text, source_type).parse();

  let program = ret.program;

  let config = Config::default();
  let mut visitor = I18NVisitor::new(&allocator, &program, "file.tsx", &config);
  visitor.options.trans_keep_basic_html_nodes_for = Some(vec!["br".into(), "strong".into(), "i".into(), "p".into()]);
  visitor.visit_program(&program);
  visitor.entries
}

mod t_function {
  use super::*;

  #[test_log::test]
  fn should_parse_t_with_options_and_ns_defined_in_variable() {
    // language=javascript
    let source_text = "const ns = 'ns'; const title = t('toast.title', undefined, {namespace: ns});";
    let keys = parse(source_text);

    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "ns")]);
  }

  #[test_log::test]
  fn should_parse_t_with_key_only() {
    // language=javascript
    let source_text = "const title = t('toast.title');";
    let keys = parse(source_text);

    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::empty("toast.title")]);
  }

  #[test_log::test]
  fn should_parse_t_with_options() {
    // language=javascript
    let source_text = "const title = t('toast.title', 'default_value', {namespace: 'ns'});";
    let keys = parse(source_text);

    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new("toast.title", "default_value", "ns")]);
  }

  #[test_log::test]
  fn should_parse_t_with_default_value() {
    // language=javascript
    let source_text = "const title = t('toast.title', 'nns');";
    let keys = parse(source_text);

    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_value("toast.title", "nns")]);
  }

  #[test_log::test]
  fn should_parse_get_fixed_t_with_ns() {
    // language=javascript
    let source_text =
      "const ns = 'ns'; const t = await i18next.getFixedT(locale, ns); const title = t('toast.title'); ";

    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "ns")]);
  }

  #[test_log::test]
  fn should_parse_t_with_default_value_and_ns_defined_in_variable() {
    // language=javascript
    let source_text = "const ns = 'ns'; const title = t('toast.title', 'default title', { namespace: ns });";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new("toast.title", "default title", "ns")]);
  }

  #[test_log::test]
  fn should_parse_t_with_no_options() {
    // language=javascript
    let source_text = "const title = t('toast.title');";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::empty("toast.title")]);
  }

  #[test_log::test]
  fn should_parse_t_with_empty_options() {
    // language=javascript
    let source_text = "const title = t('toast.title', undefined, {});";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::empty("toast.title")]);
  }

  #[test_log::test]
  fn should_parse_t_with_multiple_keys() {
    // language=javascript
    let source_text =
      "const title1 = t('toast.title1'); const title2 = t('toast.title2'); const title3 = t('toast.title3');";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 3);
    pretty_assertions::assert_eq!(
      keys,
      vec![
        Entry::empty("toast.title1"),
        Entry::empty("toast.title2"),
        Entry::empty("toast.title3")
      ]
    );
  }

  #[test_log::test]
  fn should_parse_t_with_same_key_multiple_times() {
    // language=javascript
    let source_text =
      "const title1 = t('toast.title'); const title2 = t('toast.title'); const title3 = t('toast.title');";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 3);
    pretty_assertions::assert_eq!(
      keys,
      vec![
        Entry::empty("toast.title"),
        Entry::empty("toast.title"),
        Entry::empty("toast.title")
      ]
    );
  }

  #[test_log::test]
  fn should_parse_t_with_value() {
    // language=javascript
    let source_text = "const title = t('toast.title', {defaultValue: 'Attempt {{num}}', num: 0});";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_value("toast.title", "Attempt {{num}}")]);
  }

  #[test_log::test]
  fn should_parse_t_with_count_literal_spread() {
    // language=javascript
    let source_text = "const count = 1; const title = t('toast.title', undefined, { count });";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::empty("toast.title")]);
    let el = keys.first().unwrap();
    assert!(el.has_count);
  }

  #[test_log::test]
  fn should_parse_t_with_count_literal() {
    // language=javascript
    let source_text = "const count = 1; const title = t('toast.title', undefined, {count: count});";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::empty("toast.title")]);
    let el = keys.first().unwrap();
    assert!(el.has_count);
  }

  #[test_log::test]
  fn should_parse_t_with_count_numeric() {
    // language=javascript
    let source_text = "const title = t('toast.title', undefined, {count: 1});";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::empty("toast.title")]);
    let el = keys.first().unwrap();
    assert!(el.has_count);
  }

  #[test_log::test]
  fn should_parse_t_with_count_arg() {
    // language=javascript
    let source_text = "const title = (count: number) => t('toast.title', undefined, {count: count});";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::empty("toast.title")]);
    let el = keys.first().unwrap();
    assert!(el.has_count);
  }

  #[test_log::test]
  fn should_parse_t_with_count_arg_spread() {
    // language=javascript
    let source_text = "const title = (count: number) => t('toast.title', undefined, {count});";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::empty("toast.title")]);
    let el = keys.first().unwrap();
    assert!(el.has_count);
  }

  #[test_log::test]
  fn should_parse_t_with_namespace_from_name_first_with_t() {
    // language=javascript
    let source_text =
      "const t = useTranslation('other_override'); const title = t('namespace:toast.title', {ns: 'override'});";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "namespace")]);
  }

  #[test_log::test]
  fn should_parse_t_with_namespace_from_name_first() {
    // language=javascript
    let source_text = "const title = t('namespace:toast.title', {ns: 'override'});";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "namespace")]);
  }

  #[test_log::test]
  fn should_parse_t_with_namespace_from_name() {
    // language=javascript
    let source_text = "const title = t('namespace:toast.title');";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "namespace")]);
  }

  #[test_log::test]
  fn should_parse_t_without_default_value_and_namespace() {
    // language=javascript
    let source_text = "const title = t('toast.title', {ns: 'namespace'});";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "namespace")]);
  }

  #[test_log::test]
  fn should_parse_t_with_default_value_and_namespace() {
    // language=javascript
    let source_text = "const title = t('toast.title', 'nns', {ns: 'namespace'});";
    let keys = parse(source_text);

    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new("toast.title", "nns", "namespace")]);
  }

  #[test_log::test]
  fn should_parse_t_with_default_value_and_namespace_2() {
    // language=javascript
    let source_text = "const title = t('preview.error.text', 'An error has occurred while generating the preview.\\nPlease try again.', { ns: 'invoice', })";
    let keys = parse(source_text);

    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(
      keys,
      vec![Entry::new(
        "preview.error.text",
        "An error has occurred while generating the preview.\nPlease try again.",
        "invoice"
      )]
    );
  }

  #[test_log::test]
  fn should_parse_t_with_default_value_and_namespace_3() {
    let source_text = "context.showToast({
      title: t('preview.error.title', 'Error', { ns: 'invoice' }),
      text: t('preview.error.text', 'An error has occurred while generating the preview.\\nPlease try again.', {
        ns: 'invoice',
      }),
      variant: 'destructive',
      iconType: 'invoice',
    });";
    let keys = parse(source_text);

    pretty_assertions::assert_eq!(keys.len(), 2);
    pretty_assertions::assert_eq!(
      keys,
      vec![
        Entry::new("preview.error.title", "Error", "invoice"),
        Entry::new(
          "preview.error.text",
          "An error has occurred while generating the preview.\nPlease try again.",
          "invoice"
        ),
      ]
    );
  }

  #[test_log::test]
  #[should_panic]
  fn should_parse_t_with_ns_defined_as_template_string() {
    // language=javascript
    let source_text = "const ns = 'ns'; const title = t(`${ns}:toast.title`, undefined);";
    let keys = parse(source_text);

    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "ns")]);
  }

  #[test_log::test]
  fn should_parse_t_with_ns_defined_in_clone_instance() {
    // language=javascript
    let source_text = "const ns = 'ns'; const { t } = i18next.cloneInstance({ ns }); const title = t('toast.title');";
    let keys = parse(source_text);

    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_ns("toast.title", "ns")]);
  }
}

mod translation_component {
  use super::*;

  #[test_log::test]
  fn should_extract_keys_from_render_props() {
    // language=javascript
    let source_text = "<Translation>{(t) => <>{t('first', 'Main')}{t('second')}</>}</Translation>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 2);
    pretty_assertions::assert_eq!(
      keys,
      vec![Entry::new_with_value("first", "Main"), Entry::empty("second")]
    );
  }

  #[test_log::test]
  #[should_panic] // todo: fix this test
  fn should_extract_ns_from_translation_with_render_prop() {
    // language=javascript
    let source_text = "<Translation ns='foo'>{(t) => t('first')}</Translation>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_ns("first", "foo")]);
  }
}

mod trans_component {
  use super::*;

  #[test_log::test]
  fn should_extract_default_value_from_string_litteral_prop() {
    // language=javascript
    let source_text = "<Trans i18nKey='first' defaults='test-value'>should be ignored</Trans>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_value("first", "test-value")]);
  }

  #[test_log::test]
  fn should_extract_default_value_from_interpolated_string_prop() {
    // language=javascript
    let source_text = "<Trans i18nKey='first' defaults={'test-value'}>should be ignored</Trans>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_value("first", "test-value")]);
  }

  #[test_log::test]
  fn should_extract_key_from_self_closing() {
    // language=javascript
    let source_text = "<Trans i18nKey='first' />";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys, vec![Entry::empty("first")]);
  }

  #[test_log::test]
  fn should_extract_ns_when_in_key() {
    // language=javascript
    let source_text = "<Trans i18nKey='ns:first' />";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_ns("first", "ns")]);
  }

  #[test_log::test]
  fn should_extract_ns_when_in_key_and_specified() {
    // language=javascript
    let source_text = "<Trans i18nKey='ns2:first' ns='ns' />";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_ns("first", "ns2")]);
  }

  #[test_log::test]
  #[should_panic] // todo: fix this test
  fn should_format_interpolations_correctly() {
    // language=javascript
    let source_text = "<Trans count={count}>{{ key: property, format: 'number' }}</Trans>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_value("{{key, number}}", "{{key, number}}")]);
  }

  #[test_log::test]
  #[should_panic] // todo: fix this test
  fn should_strip_invalid_interpolations() {
    // language=javascript
    let source_text = "<Trans count={count}>before{{ key1, key2 }}after</Trans>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys, vec![Entry::new_with_value("beforeafter", "beforeafter")]);
  }

  #[test_log::test]
  fn should_not_add_empty_for_self_closing_tags() {
    // language=javascript
    let source_text = "<Trans count={count}/>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 0);
    pretty_assertions::assert_eq!(keys, vec![]);
  }

  #[test_log::test]
  fn should_not_add_empty_for_empty_tags() {
    // language=javascript
    let source_text = "<Trans count={count}></Trans>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 0);
    pretty_assertions::assert_eq!(keys, vec![]);
  }

  #[test_log::test]
  #[should_panic] // todo: fix this test
  fn should_erases_tags_from_content() {
    // language=javascript
    let source_text = "<Trans>a<b test={'</b>'}>c<c>z</c></b>{d}<br stuff={y}/></Trans>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 0);
    let first = keys.first().unwrap();
    pretty_assertions::assert_eq!(first.value, Some("a<1>c<1>z</1></1>{d}<3></3>".into()));
  }

  #[test_log::test]
  #[should_panic] // todo: fix this test
  fn should_skips_dynamic_children() {
    // language=javascript
    let source_text =
      "<Trans>My dogs are named: <ul i18nIsDynamicList>{['rupert', 'max'].map(dog => (<li>{dog}</li>))}</ul></Trans>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 0);
    let first = keys.first().unwrap();
    pretty_assertions::assert_eq!(first.value, Some("My dogs are named: <1></1>".into()));
  }

  #[test_log::test]
  #[should_panic] // todo: fix this test
  fn should_handle_spread_attributes() {
    // language=javascript
    let source_text = "<Trans>My dog is named: <span {...styles}>Spot</span></Trans>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 0);
    let first = keys.first().unwrap();
    pretty_assertions::assert_eq!(first.value, Some("My dog is named: <1>Spot</1>".into()));
  }

  #[test_log::test]
  #[should_panic] // todo: fix this test
  fn should_erases_comment_expressions() {
    // language=javascript
    let source_text = "<Trans>{/* some comment */}Some Content</Trans>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 0);
    let first = keys.first().unwrap();
    pretty_assertions::assert_eq!(first.value, Some("Some Content".into()));
  }

  #[test_log::test]
  #[should_panic] // todo: fix this test
  fn should_handles_jsx_fragments() {
    // language=javascript
    let source_text = "<><Trans i18nKey='first' /></>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 0);
    pretty_assertions::assert_eq!(keys, vec![Entry::empty("first")]);
  }

  #[test_log::test]
  #[should_panic] // todo: fix this test
  fn should_interpolates_literal_string_values() {
    // language=javascript
    let source_text = "<Trans>Some{' '}Interpolated {'Content'}</Trans>";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 0);
    let first = keys.first().unwrap();
    pretty_assertions::assert_eq!(first.value, Some("Some Interpolated Content".into()));
  }

  #[test_log::test]
  fn should_parse_jsx_with_ns_defined_in_variable() {
    // language=javascript
    let source_text = "const ns = 'ns'; const el = <Trans ns={ns} i18nKey='dialog.title'>Reset password</Trans>;";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
  }

  #[test_log::test]
  fn should_parse_jsx_with_ns() {
    // language=javascript
    let source_text = "const el = <Trans ns='ns' i18nKey='dialog.title'>Reset password</Trans>;";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
  }

  #[test_log::test]
  fn should_parse_jsx_with_template_translated() {
    // language=javascript
    let source_text = "const Comp = () => <i>Reset password</i>; const el = <Trans ns='ns' i18nKey='dialog.title'><Comp>Reset password</Comp></Trans>;";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new("dialog.title", "<0>Reset password</0>", "ns")]);
  }

  #[test_log::test]
  fn should_parse_jsx_with_nested_template() {
    // language=javascript
    let source_text =
      "const attempt = 0; const el = <Trans ns='ns' i18nKey='dialog.title'>Reset password {{attempt}}</Trans>;";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(
      keys,
      vec![Entry::new("dialog.title", "Reset password {{attempt}}", "ns")]
    );
  }

  #[test_log::test]
  fn should_parse_jsx_with_nested_template_object() {
    // language=javascript
    let source_text = "const attempt = 0; const el = <Trans ns='ns' i18nKey='dialog.title'>Reset password {{ attempt: attempt + 1 }}</Trans>;";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(
      keys,
      vec![Entry::new("dialog.title", "Reset password {{attempt}}", "ns")]
    );
  }

  #[test_log::test]
  fn should_parse_jsx_with_nested_template_object_and_text_after() {
    // language=javascript
    let source_text = "const attempt = 0; const el = <Trans ns='ns' i18nKey='dialog.title'>Attempt {{ attempt: attempt + 1 }} on 10</Trans>;";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(
      keys,
      vec![Entry::new("dialog.title", "Attempt {{attempt}} on 10", "ns")]
    );
  }

  #[test_log::test]
  fn should_parse_jsx_with_self_closing_element() {
    // language=javascript
    let source_text = "const el = <Trans ns='ns' i18nKey='dialog.title'>Reset password<br /></Trans>;";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password<1></1>", "ns")]);
  }

  #[test_log::test]
  fn should_parse_jsx_with_template_removed_when_unspecified() {
    // language=javascript
    let source_text = "const el = <Trans ns='ns' i18nKey='dialog.title'><i>Reset password</i></Trans>;";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new("dialog.title", "<0>Reset password</0>", "ns")]);
  }

  #[test_log::test]
  fn should_parse_jsx_with_template_kept() {
    // language=javascript
    let source_text = "const el = <Trans ns='ns' i18nKey='dialog.title'><i>Reset password</i></Trans>;";
    let keys = parse_with_options(source_text);
    pretty_assertions::assert_eq!(keys.len(), 1);
    pretty_assertions::assert_eq!(keys, vec![Entry::new("dialog.title", "<i>Reset password</i>", "ns")]);
  }

  #[test_log::test]
  fn should_parse_jsx_and_return_nothing_on_bad_components() {
    // language=javascript
    let source_text = "const el = <Trad ns='ns' i18nKey='dialog.title'><i>Reset password</i></Trad>;";
    let keys = parse(source_text);
    pretty_assertions::assert_eq!(keys.len(), 0);
  }

  mod count {
    use super::*;
    #[test_log::test]
    fn should_parse_jsx_with_count_identifier() {
      // language=javascript
      let source_text =
        "const count = 2; const el = <Trans ns='ns' i18nKey='dialog.title' count={count}>Reset password</Trans>;";
      let keys = parse(source_text);
      pretty_assertions::assert_eq!(keys.len(), 1);
      pretty_assertions::assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
      let le = keys.first().unwrap();
      assert!(le.has_count);
    }

    #[test_log::test]
    fn should_parse_jsx_with_count_numeral() {
      // language=javascript
      let source_text = "const el = <Trans ns='ns' i18nKey='dialog.title' count={2}>Reset password</Trans>;";
      let keys = parse(source_text);
      pretty_assertions::assert_eq!(keys.len(), 1);
      pretty_assertions::assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);
      let le = keys.first().unwrap();
      assert!(le.has_count);
    }

    #[test_log::test]
    fn should_parse_jsx_with_count_double_reference() {
      // language=javascript
      let source_text =
        "const a = 2; const b = a; const el = <Trans ns='ns' i18nKey='dialog.title' count={b}>Reset password</Trans>;";
      let keys = parse(source_text);
      pretty_assertions::assert_eq!(keys.len(), 1);
      pretty_assertions::assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);

      let le = keys.first().unwrap();
      assert!(le.has_count);
    }

    #[test_log::test]
    fn should_parse_jsx_with_count_from_arg() {
      // language=javascript
      let source_text =
        "const el = (count: number) => <Trans ns='ns' i18nKey='dialog.title' count={count}>Reset password</Trans>;";
      let keys = parse(source_text);
      pretty_assertions::assert_eq!(keys.len(), 1);
      pretty_assertions::assert_eq!(keys, vec![Entry::new("dialog.title", "Reset password", "ns")]);

      let le = keys.first().unwrap();
      assert!(le.has_count);
    }
  }

  mod context {
    use super::*;

    mod t_function {
      use super::*;

      #[test_log::test]
      fn should_parse_context_from_string_arg_type_alias() {
        // language=javascript
        let source_text = "type Ctx = 'male' | 'female'; function El(val: Ctx) {
            const {t} = useTranslation('ns');
            return t('dialog.title', 'Reset password', { context: val });
            }";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!("male".into(), "female".into())
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_props_arg_function() {
        // language=javascript
        let source_text = "function El({ val }: {val: 'male' | 'female'}) {
            const {t} = useTranslation('ns');
            return t('dialog.title', 'Reset password', { context: val });
}";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!("male".into(), "female".into())
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_string_arg_function() {
        // language=javascript
        let source_text = "function El(val: 'male' | 'female') {
            const {t} = useTranslation('ns');
            return t('dialog.title', 'Reset password', { context: val });
}";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!("male".into(), "female".into())
          )]
        );
      }

      #[test_log::test(ignore = "reason")]
      fn should_parse_context_from_string_arg_const_function() {
        // language=javascript
        let source_text = "const El = (val: 'male' | 'female') => <Trans ns='ns' i18nKey='dialog.title' context={val}>Reset password</Trans>;";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!("male".into(), "female".into())
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_variable_type() {
        // language=javascript
        let source_text = "const getSex = () => 'male'; const val: 'male' | 'female' = getSex();  const {t} = useTranslation('ns'); const el =  t('dialog.title', 'Reset password', { context: val });";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!("male".into(), "female".into())
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_remote_type_complex() {
        // language=javascript
        let source_text = "
                  type Users = Array<{
                      role: 'admin' | 'member' | 'owner',
                  }>;
                  type Role = Users[number]['role'];
                  export function InvitationEmail() {
            const role = invitee.role as Role;
           const {t} = useTranslation('ns');
           return t('role', 'Role', { context: role });
          }
          ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_as_variable_types() {
        // language=javascript
        let source_text = "
          export function InvitationEmail() {
            const role = invitee.role as 'admin' | 'member' | 'owner' ;
            const {t} = useTranslation('ns');
            return t('role', 'Role', { context: role });
          }
          ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_variable_types() {
        // language=javascript
        let source_text = "
          export function InvitationEmail() {
            const role: 'admin' | 'member' | 'owner' = invitee.role;
            const {t} = useTranslation('ns');
            return t('role', 'Role', { context: role });
          }
          ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_variable_types_brackets_array() {
        // language=javascript
        let source_text = "
          export function InvitationEmail() {
            const role: ('admin' | 'member' | 'owner')[] = invitee.role;
            const {t} = useTranslation('ns');
            return t('role', 'Role', { context: role });
          }
          ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_variable_types_array() {
        // language=javascript
        let source_text = "
          export function InvitationEmail() {
            const role: Array<'admin' | 'member' | 'owner'> = invitee.role;
            const {t} = useTranslation('ns');
            return t('role', 'Role', { context: role });
          }
          ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_function_props() {
        // language=javascript
        let source_text = "
          export function InvitationEmail({role}: {role: 'admin' | 'member' | 'owner'}) {
            const {t} = useTranslation('ns');
            return t('role', 'Role', { context: role });
          }";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_generic_type() {
        // language=javascript
        let source_text = "
                  import type React from 'react';
          export function InvitationEmail({role}: React.PropsWithChildren<{role: 'admin' | 'member' | 'owner'}>) {
            const {t} = useTranslation('ns');
            return t('role', 'Role', { context: role });
          }
          ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_variable() {
        // language=javascript
        let source_text = "const val = 'male'; const {t} = useTranslation('ns'); const el = t('dialog.title', 'Reset password', { context: val });";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!["male".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_string_literal() {
        // language=javascript
        let source_text =
          "const {t} = useTranslation('ns'); const el = t('dialog.title', 'Reset password', { context: 'male' });";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!("male".into())
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_type_union_with_typeof() {
        // language=javascript
        let source_text = "
                  const sexKinds = ['male', 'female'] as const;
                  type SexKind = (typeof sexKinds)[number];
                  function A({kind} : {kind: SexKind}) {
                  const {t} = useTranslation('ns'); return t('dialog.title', 'Reset password', { context: kind});
                  }";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!["male".into(), "female".into()]
          ),]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_string_literal_with_multiple_entries() {
        // language=javascript
        let source_text = "
            const {t} = useTranslation('ns');
            const a = t('dialog.title', 'Reset password', { context: 'male' });
            const b = t('dialog.title', 'Reset password', { context: 'female' });
            ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(
          keys,
          vec![
            Entry::new_with_context("dialog.title", "Reset password", "ns", vec!["male".into()]),
            Entry::new_with_context("dialog.title", "Reset password", "ns", vec!["female".into()])
          ]
        );
      }
    }

    mod jsx {
      use super::*;

      #[test_log::test]
      fn should_parse_context_from_string_arg_type_alias() {
        // language=javascript
        let source_text = "type Ctx = 'male' | 'female'; function El(val: Ctx) {
            const {t} = useTranslation('ns');
            return t('dialog.title', 'Reset password', { context: val });
}";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!("male".into(), "female".into())
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_props_arg_function() {
        // language=javascript
        let source_text = "function El({ val }: {val: 'male' | 'female'}) {
            const {t} = useTranslation('ns');
            return t('dialog.title', 'Reset password', { context: val });
}";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!("male".into(), "female".into())
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_string_arg_function() {
        // language=javascript
        let source_text = "function El(val: 'male' | 'female') {
            const {t} = useTranslation('ns');
            return t('dialog.title', 'Reset password', { context: val });
}";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!("male".into(), "female".into())
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_string_arg_const_function() {
        // language=javascript
        let source_text = "const El = (val: 'male' | 'female') => <Trans ns='ns' i18nKey='dialog.title' context={val}>Reset password</Trans>;";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!("male".into(), "female".into())
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_variable_type() {
        // language=javascript
        let source_text = "const getSex = () => 'male'; const val: 'male' | 'female' = getSex(); const el = <Trans ns='ns' i18nKey='dialog.title' context={val}>Reset password</Trans>;";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!("male".into(), "female".into())
          )]
        );
      }

      #[test_log::test]
      fn test_1() {
        // language=javascript
        let source_text = "function ThemeDropdownMenu() {
            const { t } = useTranslation('ns');
            const [theme, setTheme] = useTheme();
            const preferredTheme: 'dark' | 'light' = getPreferredTheme();

            return (
              <Trans context={preferredTheme} i18nKey='theme.system' ns='ns' t={t}>
                  System theme
              </Trans>
            );
          }";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "theme.system",
            "System theme",
            "ns",
            vec!["dark".into(), "light".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_remote_type_complex() {
        // language=javascript
        let source_text = "
                  type Users = Array<{
                      role: 'admin' | 'member' | 'owner',
                  }>;
                  type Role = Users[number]['role'];
                  export function InvitationEmail() {
            const role = invitee.role as Role;
            return (
              <EmailRoot>
                <Text className='truncate'>
                  <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
                </Text>
              </EmailRoot>
            );
          }
          ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_as_variable_types() {
        // language=javascript
        let source_text = "
          export function InvitationEmail() {
            const role = invitee.role as 'admin' | 'member' | 'owner' ;
            return (
              <EmailRoot>
                <Text className='truncate'>
                  <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
                </Text>
              </EmailRoot>
            );
          }
          ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_variable_types() {
        // language=javascript
        let source_text = "
          export function InvitationEmail() {
            const role: 'admin' | 'member' | 'owner' = invitee.role;
            return (
              <EmailRoot>
                <Text className='truncate'>
                  <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
                </Text>
              </EmailRoot>
            );
          }
          ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_variable_types_brackets_array() {
        // language=javascript
        let source_text = "
          export function InvitationEmail() {
            const role: ('admin' | 'member' | 'owner')[] = invitee.role;
            return (
              <EmailRoot>
                <Text className='truncate'>
                  <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
                </Text>
              </EmailRoot>
            );
          }
          ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_variable_types_array() {
        // language=javascript
        let source_text = "
          export function InvitationEmail() {
            const role: Array<'admin' | 'member' | 'owner'> = invitee.role;
            return (
              <EmailRoot>
                <Text className='truncate'>
                  <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
                </Text>
              </EmailRoot>
            );
          }
          ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_function_props() {
        // language=javascript
        let source_text = "
          export function InvitationEmail({role}: {role: 'admin' | 'member' | 'owner'}) {
            return (
              <EmailRoot>
                <Text className='truncate'>
                  <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
                </Text>
              </EmailRoot>
            );
          }
          ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_generic_type() {
        // language=javascript
        let source_text = "
                  import type React from 'react';
          export function InvitationEmail({role}: React.PropsWithChildren<{role: 'admin' | 'member' | 'owner'}>) {
            return (
              <EmailRoot>
                <Text className='truncate'>
                  <Trans context={role} i18nKey='role' ns='ns'>Role</Trans>
                </Text>
              </EmailRoot>
            );
          }
          ";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "role",
            "Role",
            "ns",
            vec!["admin".into(), "member".into(), "owner".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_variable() {
        // language=javascript
        let source_text =
          "const val = 'male'; const el = <Trans ns='ns' i18nKey='dialog.title' context={val}>Reset password</Trans>;";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!["male".into()]
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_string_literal() {
        // language=javascript
        let source_text = "const el = <Trans ns='ns' i18nKey='dialog.title' context='male'>Reset password</Trans>;";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(keys.len(), 1);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!("male".into())
          )]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_type_union_with_typeof() {
        // language=javascript
        let source_text = "
                  const sexKinds = ['male', 'female'] as const;
                  type SexKind = (typeof sexKinds)[number];
                  function A({kind} : {kind: SexKind}) {
                  return <Trans ns='ns' i18nKey='dialog.title' context={kind}>Reset password</Trans>;
                  }";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(
          keys,
          vec![Entry::new_with_context(
            "dialog.title",
            "Reset password",
            "ns",
            vec!["male".into(), "female".into()]
          ),]
        );
      }

      #[test_log::test]
      fn should_parse_context_from_string_literal_with_multiple_entries() {
        // language=javascript
        let source_text = "const a = <Trans ns='ns' i18nKey='dialog.title' context='male'>Reset password</Trans>;const b = <Trans ns='ns' i18nKey='dialog.title' context='female'>Reset password</Trans>;";
        let keys = parse(source_text);
        pretty_assertions::assert_eq!(
          keys,
          vec![
            Entry::new_with_context("dialog.title", "Reset password", "ns", vec!["male".into()]),
            Entry::new_with_context("dialog.title", "Reset password", "ns", vec!["female".into()])
          ]
        );
      }
    }
  }
}

mod parsing {
  use serde_json::Value;

  use super::*;
  use crate::visitor::traits::oxc_custom_parser::OxcCustomParser;

  fn get_value(source_text: &str, value: &str) -> Option<Value> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path("file.tsx").unwrap();
    let ret = Parser::new(&allocator, source_text, source_type).parse();
    log::trace!("Program: {:#?}", ret.program.body);

    let program = ret.program;

    let config = Config::default();
    let visitor = I18NVisitor::new(&allocator, &program, "file.tsx", &config);
    visitor.find_identifier_value_as_serde(value)
  }

  mod local_types {
    use serde_json::json;

    use super::*;

    #[test_log::test]
    fn should_parse_union_type() {
      // language=javascript
      let source_text = "type TestType = 'admin' | 'member' | 'owner';";

      let val = get_value(source_text, "TestType");
      assert_eq!(
        val,
        Some(Value::Array(vec!["admin".into(), "member".into(), "owner".into()]))
      )
    }

    #[test_log::test]
    fn should_parse_array_type() {
      // language=javascript
      let source_text = "type TestType = Array<'admin' | 'member' | 'owner'>;";

      let val = get_value(source_text, "TestType");
      assert_eq!(
        val,
        Some(Value::Array(vec!["admin".into(), "member".into(), "owner".into()]))
      )
    }

    #[test_log::test]
    fn should_parse_brackets_array_type() {
      // language=javascript
      let source_text = "type TestType = ('admin' | 'member' | 'owner')[];";

      let val = get_value(source_text, "TestType");
      assert_eq!(
        val,
        Some(Value::Array(vec!["admin".into(), "member".into(), "owner".into()]))
      )
    }

    #[test_log::test]
    fn should_parse_object_type() {
      // language=javascript
      let source_text = "type TestType = {
        a: 'b',
      };";

      let val = get_value(source_text, "TestType");
      assert_eq!(val, Some(json!({"a": "b"})))
    }
  }

  mod reference_types {
    use super::*;

    #[test_log::test]
    fn should_parse_union_type() {
      // language=javascript
      let source_text = "
          type Role = 'admin' | 'member' | 'owner';
          type TestType = Role;
        ";

      let val = get_value(source_text, "TestType");
      assert_eq!(
        val,
        Some(Value::Array(vec!["admin".into(), "member".into(), "owner".into()]))
      )
    }

    #[test_log::test]
    fn should_parse_array_type() {
      // language=javascript
      let source_text = "
          type Role = 'admin' | 'member' | 'owner';
          type TestType = Array<Role>;
        ";

      let val = get_value(source_text, "TestType");
      assert_eq!(
        val,
        Some(Value::Array(vec!["admin".into(), "member".into(), "owner".into()]))
      )
    }

    #[test_log::test]
    fn should_parse_brackets_array_type() {
      // language=javascript
      let source_text = "
          type Role = 'admin' | 'member' | 'owner';
          type TestType = Role[];
        ";

      let val = get_value(source_text, "TestType");
      assert_eq!(
        val,
        Some(Value::Array(vec!["admin".into(), "member".into(), "owner".into()]))
      )
    }

    #[test_log::test]
    fn should_parse_bracket_referenced_fields() {
      // language=javascript
      let source_text = "
          type User = {
            role: 'admin' | 'member' | 'owner';
          };
          type TestType = User['role'];
        ";

      let val = get_value(source_text, "TestType");
      assert_eq!(
        val,
        Some(Value::Array(vec!["admin".into(), "member".into(), "owner".into()]))
      )
    }

    #[test_log::test]
    fn should_parse_bracket_referenced_array_fields() {
      // language=javascript
      let source_text = "
          type Users = Array<{
              role: 'admin' | 'member' | 'owner',
          }>;
          type TestType = Users[number]['role'];
        ";

      let val = get_value(source_text, "TestType");
      assert_eq!(
        val,
        Some(Value::Array(vec!["admin".into(), "member".into(), "owner".into()]))
      )
    }

    #[test_log::test]
    fn should_parse_indexed_array() {
      let source_text = r#"
        declare const roles: readonly ["admin", "member", "owner"];
        type TestType = (typeof roles)[number];
        "#;
      let val = get_value(source_text, "TestType");
      assert_eq!(
        val,
        Some(Value::Array(vec!["admin".into(), "member".into(), "owner".into()]))
      )
    }
  }
}
