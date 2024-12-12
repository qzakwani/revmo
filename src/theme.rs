use iced::Theme;
use once_cell::sync::Lazy;

pub static THEMES: Lazy<[Theme; 44]> = Lazy::new(|| {
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

static THEMES_LEN: Lazy<usize> = Lazy::new(|| THEMES.len());

pub fn get_theme(i: usize) -> &'static Theme {
    &THEMES[i]
}

pub fn forward_theme(i: usize) -> (&'static Theme, usize) {
    if i == THEMES_LEN.clone() - 1 {
        return (&THEMES[0], 0);
    }
    (&THEMES[i + 1], i + 1)
}

pub fn backward_theme(i: usize) -> (&'static Theme, usize) {
    if i == 0 {
        return (&THEMES[THEMES_LEN.clone() - 1], THEMES_LEN.clone() - 1);
    }
    (&THEMES[i - 1], i - 1)
}
