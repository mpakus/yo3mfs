use yo3mfs::settings::{self, Settings};

#[test]
fn english_is_default_and_old_preferences_keep_their_background() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, r#"{"version":1,"background":[17,34,51]}"#).unwrap();
    let (old, warning) = settings::read(&path);
    assert!(warning.is_none());
    assert_eq!(old.background, [17, 34, 51]);
    assert_eq!(serde_json::to_value(old).unwrap()["language"], "en");
    assert_eq!(
        serde_json::to_value(Settings::default()).unwrap()["language"],
        "en"
    );
}

#[test]
fn russian_choice_survives_save_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let value = serde_json::json!({"version":1,"background":[40,50,60],"language":"ru"});
    let settings: Settings = serde_json::from_value(value.clone()).unwrap();
    settings::save(&path, &settings).unwrap();
    let (restored, warning) = settings::read(&path);
    assert!(warning.is_none());
    assert_eq!(serde_json::to_value(restored).unwrap(), value);
}

#[test]
fn an_unknown_language_falls_back_without_resetting_background() {
    let settings: Settings = serde_json::from_value(serde_json::json!({
        "version":1, "background":[17,34,51], "language":"xx"
    }))
    .unwrap();
    assert_eq!(settings.background, [17, 34, 51]);
    assert_eq!(settings.language, yo3mfs::i18n::Language::English);
}

#[test]
fn existing_diagnostics_can_switch_languages_without_reimporting() {
    use yo3mfs::{
        i18n::{Key, Language, Message},
        import::ImportError,
    };
    let error = ImportError::Invalid(Message::with(Key::MissingPart, ["деталь.model"]));
    assert_eq!(
        error.localized(Language::English),
        "Invalid 3MF: Missing part: деталь.model"
    );
    assert_eq!(
        error.localized(Language::Russian),
        "Некорректный 3MF: Не найдена часть: деталь.model"
    );
    let warning = Message::with(Key::ExcludedWarning, [3]);
    assert!(
        warning
            .render(Language::English)
            .starts_with("Helper parts excluded: 3.")
    );
    assert!(warning.render(Language::Russian).contains('3'));
}
