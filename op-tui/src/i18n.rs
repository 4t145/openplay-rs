use std::sync::OnceLock;

use fluent::concurrent::FluentBundle;
use fluent::{FluentArgs, FluentResource};
use fluent_langneg::{negotiate_languages, NegotiationStrategy};
use unic_langid::LanguageIdentifier;

/// Available locales
const AVAILABLE_LOCALES: &[&str] = &["en", "zh-CN"];
const DEFAULT_LOCALE: &str = "en";

/// Embedded FTL resources
const EN_MAIN: &str = include_str!("../locales/en/main.ftl");
const EN_DOUDIZHU: &str = include_str!("../locales/en/doudizhu.ftl");
const ZH_CN_MAIN: &str = include_str!("../locales/zh-CN/main.ftl");
const ZH_CN_DOUDIZHU: &str = include_str!("../locales/zh-CN/doudizhu.ftl");

type Bundle = FluentBundle<FluentResource>;

static BUNDLE: OnceLock<Bundle> = OnceLock::new();

/// Initialize the i18n system with the given locale preference.
/// Falls back through: requested -> system locale -> "en"
pub fn init(requested_locale: Option<&str>) {
    BUNDLE.get_or_init(|| create_bundle(requested_locale));
}

/// Translate a message by id (no arguments).
pub fn t(msg_id: &str) -> String {
    let bundle = BUNDLE
        .get()
        .expect("i18n not initialized; call i18n::init() first");
    format_message(bundle, msg_id, None)
}

/// Translate a message by id with arguments.
pub fn t_args(msg_id: &str, args: &FluentArgs) -> String {
    let bundle = BUNDLE
        .get()
        .expect("i18n not initialized; call i18n::init() first");
    format_message(bundle, msg_id, Some(args))
}

fn format_message(bundle: &Bundle, msg_id: &str, args: Option<&FluentArgs>) -> String {
    let Some(msg) = bundle.get_message(msg_id) else {
        return msg_id.to_string();
    };
    let Some(pattern) = msg.value() else {
        return msg_id.to_string();
    };
    let mut errors = vec![];
    let result = bundle.format_pattern(pattern, args, &mut errors);
    // Strip Unicode bidi isolation characters (FSI/PDI) that Fluent adds around placeables.
    // These are invisible but can cause display issues in terminal UIs.
    result.replace('\u{2068}', "").replace('\u{2069}', "")
}

fn create_bundle(requested_locale: Option<&str>) -> Bundle {
    // Determine the best locale
    let available: Vec<LanguageIdentifier> = AVAILABLE_LOCALES
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let default_locale: LanguageIdentifier = DEFAULT_LOCALE.parse().unwrap();

    // Build the requested list: explicit > system > default
    let mut requested: Vec<LanguageIdentifier> = Vec::new();
    if let Some(req) = requested_locale {
        if let Ok(lid) = req.parse::<LanguageIdentifier>() {
            requested.push(lid);
        }
    }
    if let Some(sys) = sys_locale::get_locale() {
        if let Ok(lid) = sys.parse::<LanguageIdentifier>() {
            requested.push(lid);
        }
    }
    requested.push(default_locale.clone());

    let requested_refs: Vec<&LanguageIdentifier> = requested.iter().collect();
    let available_refs: Vec<&LanguageIdentifier> = available.iter().collect();

    let default_ref = &default_locale;
    let negotiated = negotiate_languages(
        &requested_refs,
        &available_refs,
        Some(&default_ref),
        NegotiationStrategy::Filtering,
    );

    let best_locale = negotiated
        .first()
        .map(|lang| (**lang).clone())
        .unwrap_or_else(|| default_locale.clone());

    let locale_str = best_locale.to_string();
    tracing::info!("Using locale: {}", locale_str);

    let mut bundle = FluentBundle::new_concurrent(vec![best_locale.clone()]);

    // Load resources for the selected locale
    let (main_ftl, doudizhu_ftl) = match locale_str.as_str() {
        "zh-CN" => (ZH_CN_MAIN, ZH_CN_DOUDIZHU),
        _ => (EN_MAIN, EN_DOUDIZHU),
    };

    if let Ok(resource) = FluentResource::try_new(main_ftl.to_string()) {
        let _ = bundle.add_resource(resource);
    }
    if let Ok(resource) = FluentResource::try_new(doudizhu_ftl.to_string()) {
        let _ = bundle.add_resource(resource);
    }

    // If not English, also add English as fallback.
    // add_resource_overriding only adds keys that are missing in the primary locale.
    if locale_str != "en" {
        if let Ok(resource) = FluentResource::try_new(EN_MAIN.to_string()) {
            let _ = bundle.add_resource_overriding(resource);
        }
        if let Ok(resource) = FluentResource::try_new(EN_DOUDIZHU.to_string()) {
            let _ = bundle.add_resource_overriding(resource);
        }
    }

    bundle
}
