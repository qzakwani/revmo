use iced::Theme;
use std::sync::LazyLock;

pub static THEMES: LazyLock<[Theme; 44]> = LazyLock::new(|| {
    [
        Theme::Light,
        Theme::custom("Nude Light".to_string(), Theme::Light.palette()),
        Theme::Dark,
        Theme::custom("Nude Dark".to_string(), Theme::Dark.palette()),
        Theme::Dracula,
        Theme::custom("Nude Dracula".to_string(), Theme::Dracula.palette()),
        Theme::Nord,
        Theme::custom("Nude Nord".to_string(), Theme::Nord.palette()),
        Theme::SolarizedLight,
        Theme::custom(
            "Nude Solarized Light".to_string(),
            Theme::SolarizedLight.palette(),
        ),
        Theme::SolarizedDark,
        Theme::custom(
            "Nude Solarized Dark".to_string(),
            Theme::SolarizedDark.palette(),
        ),
        Theme::GruvboxLight,
        Theme::custom(
            "Nude Gruvbox Light".to_string(),
            Theme::GruvboxLight.palette(),
        ),
        Theme::GruvboxDark,
        Theme::custom(
            "Nude Gruvbox Dark".to_string(),
            Theme::GruvboxDark.palette(),
        ),
        Theme::CatppuccinLatte,
        Theme::custom(
            "Nude Catppuccin Latte".to_string(),
            Theme::CatppuccinLatte.palette(),
        ),
        Theme::CatppuccinFrappe,
        Theme::custom(
            "Nude Catppuccin Frappe".to_string(),
            Theme::CatppuccinFrappe.palette(),
        ),
        Theme::CatppuccinMacchiato,
        Theme::custom(
            "Nude Catppuccin Macchiato".to_string(),
            Theme::CatppuccinMacchiato.palette(),
        ),
        Theme::CatppuccinMocha,
        Theme::custom(
            "Nude Catppuccin Mocha".to_string(),
            Theme::CatppuccinMocha.palette(),
        ),
        Theme::TokyoNight,
        Theme::custom("Nude Tokyo Night".to_string(), Theme::TokyoNight.palette()),
        Theme::TokyoNightStorm,
        Theme::custom(
            "Nude Tokyo Night Storm".to_string(),
            Theme::TokyoNightStorm.palette(),
        ),
        Theme::TokyoNightLight,
        Theme::custom(
            "Nude Tokyo Night Light".to_string(),
            Theme::TokyoNightLight.palette(),
        ),
        Theme::KanagawaWave,
        Theme::custom(
            "Nude Kanagawa Wave".to_string(),
            Theme::KanagawaWave.palette(),
        ),
        Theme::KanagawaDragon,
        Theme::custom(
            "Nude Kanagawa Dragon".to_string(),
            Theme::KanagawaDragon.palette(),
        ),
        Theme::KanagawaLotus,
        Theme::custom(
            "Nude Kanagawa Lotus".to_string(),
            Theme::KanagawaLotus.palette(),
        ),
        Theme::Moonfly,
        Theme::custom("Nude Moonfly".to_string(), Theme::Moonfly.palette()),
        Theme::Nightfly,
        Theme::custom("Nude Nightfly".to_string(), Theme::Nightfly.palette()),
        Theme::Oxocarbon,
        Theme::custom("Nude Oxocarbon".to_string(), Theme::Oxocarbon.palette()),
        Theme::Ferra,
        Theme::custom("Nude Ferra".to_string(), Theme::Ferra.palette()),
    ]
});

pub fn default_theme() -> &'static Theme {
    &THEMES[0]
}

/// The theme with the given name, or the default one.
pub fn by_name(name: &str) -> &'static Theme {
    THEMES
        .iter()
        .find(|t| t.to_string() == name)
        .unwrap_or_else(default_theme)
}

fn position(current: &Theme) -> usize {
    let name = current.to_string();
    THEMES
        .iter()
        .position(|t| t.to_string() == name)
        .unwrap_or(0)
}

pub fn next(current: &Theme) -> &'static Theme {
    &THEMES[(position(current) + 1) % THEMES.len()]
}

pub fn previous(current: &Theme) -> &'static Theme {
    &THEMES[(position(current) + THEMES.len() - 1) % THEMES.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_by_name() {
        let first = default_theme();
        let last = &THEMES[THEMES.len() - 1];
        assert_eq!(next(last).to_string(), first.to_string());
        assert_eq!(previous(first).to_string(), last.to_string());
        assert_eq!(
            by_name(&THEMES[3].to_string()).to_string(),
            THEMES[3].to_string()
        );
        assert_eq!(by_name("No Such Theme").to_string(), first.to_string());
    }
}
