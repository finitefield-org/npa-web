use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use ironframe::{
    generator::{self, GeneratorConfig, VariantOverrides},
    scanner,
};

use crate::render;

const CLASS_SAFELIST: &[&str] = &[
    "npa-theme",
    "grid-cols-[8rem_1fr]",
    "grid-cols-[10rem_1fr_6rem]",
];
const THEME_UTILITY_CLASS: &str = "npa-theme";
const THEME_UTILITY_BODY: &str = "\
font-family:system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;\
color:#111827;\
background:#ffffff;\
--spacing:0.25rem;\
--tw-border-style:solid;\
--font-mono:ui-monospace,SFMono-Regular,Menlo,Consolas,\"Liberation Mono\",\"Courier New\",monospace;\
--text-xs:0.75rem;\
--text-xs--line-height:1rem;\
--text-sm:0.875rem;\
--text-sm--line-height:1.25rem;\
--text-lg:1.125rem;\
--text-lg--line-height:1.75rem;\
--font-weight-medium:500;\
--font-weight-semibold:600";

static APP_CSS: OnceLock<String> = OnceLock::new();

pub fn app_css() -> &'static str {
    APP_CSS.get_or_init(generate_app_css).as_str()
}

pub fn generate_app_css() -> String {
    let classes = template_classes();
    let config = generator_config();
    let overrides = variant_overrides();
    let result = generator::generate_with_overrides(&classes, &config, Some(&overrides));

    generator::emit_css(&result)
}

pub(crate) fn template_classes() -> Vec<String> {
    let mut classes = BTreeSet::new();

    for source in render::TEMPLATE_SOURCES {
        classes.extend(scanner::extract_classes(source));
    }
    classes.extend(CLASS_SAFELIST.iter().map(|class| (*class).to_string()));

    classes.into_iter().collect()
}

fn generator_config() -> GeneratorConfig {
    GeneratorConfig {
        minify: false,
        colors: BTreeMap::new(),
    }
}

fn variant_overrides() -> VariantOverrides {
    VariantOverrides {
        responsive_breakpoints: vec![],
        container_breakpoints: vec![],
        dark_variant_selector: None,
        custom_variant_selectors: vec![],
        custom_utilities: vec![(
            THEME_UTILITY_CLASS.to_string(),
            THEME_UTILITY_BODY.to_string(),
        )],
        theme_variable_values: vec![],
        global_theme_reset: false,
        disabled_namespaces: vec![],
        disabled_color_families: vec![],
        declared_theme_vars: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_template_classes_and_safelist() {
        let classes = template_classes();

        for expected in [
            "npa-theme",
            "grid",
            "lg:grid-cols-2",
            "lg:grid-cols-3",
            "grid-cols-[8rem_1fr]",
            "grid-cols-[10rem_1fr_6rem]",
            "whitespace-pre-wrap",
            "break-all",
            "sr-only",
        ] {
            assert!(
                classes.iter().any(|class| class == expected),
                "missing class {expected}"
            );
        }
    }

    #[test]
    fn generates_css_for_first_screen_and_partials() {
        let css = generate_app_css();

        for expected in [
            ".npa-theme",
            "--spacing: 0.25rem",
            ".grid",
            "display: grid",
            ".lg\\:grid-cols-2",
            "@media (width >= 64rem)",
            ".lg\\:grid-cols-3",
            "grid-template-columns: repeat(3, minmax(0, 1fr))",
            ".grid-cols-\\[8rem_1fr\\]",
            "grid-template-columns: 8rem 1fr",
            ".grid-cols-\\[10rem_1fr_6rem\\]",
            "grid-template-columns: 10rem 1fr 6rem",
            ".font-mono",
            "font-family: var(--font-mono)",
            ".whitespace-pre-wrap",
            ".break-all",
        ] {
            assert!(css.contains(expected), "missing CSS fragment {expected}");
        }
    }

    #[test]
    fn cached_app_css_matches_generated_css() {
        assert_eq!(app_css(), generate_app_css());
    }
}
