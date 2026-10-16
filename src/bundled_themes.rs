//! The themes that come with revmo, on top of the two in
//! [`crate::theme::built_in`]. To add or change one, edit [`BUNDLED`]: give
//! the three required colors, plus any others in `other` by their theme
//! file key; the rest are worked out as for a theme file.

/// A theme in the same terms as a `.theme` file.
pub struct Bundled {
    pub name: &'static str,
    pub background: u32,
    pub text: u32,
    pub accent: u32,
    /// Optional colors, by theme file key (e.g. `("Danger", 0xff5555)`).
    pub other: &'static [(&'static str, u32)],
}

/// Sorted by name.
pub const BUNDLED: [Bundled; 22] = [
    Bundled {
        name: "Catppuccin Frappé",
        background: 0x303446,
        text: 0xc6d0f5,
        accent: 0x8caaee,
        other: &[("Danger", 0xe78284), ("Success", 0xa6d189)],
    },
    Bundled {
        name: "Catppuccin Latte",
        background: 0xeff1f5,
        text: 0x4c4f69,
        accent: 0x1e66f5,
        other: &[("Danger", 0xd20f39), ("Success", 0x40a02b)],
    },
    Bundled {
        name: "Catppuccin Macchiato",
        background: 0x24273a,
        text: 0xcad3f5,
        accent: 0x8aadf4,
        other: &[("Danger", 0xed8796), ("Success", 0xa6da95)],
    },
    Bundled {
        name: "Catppuccin Mocha",
        background: 0x1e1e2e,
        text: 0xcdd6f4,
        accent: 0x89b4fa,
        other: &[("Danger", 0xf38ba8), ("Success", 0xa6e3a1)],
    },
    Bundled {
        name: "Dark",
        background: 0x2b2d31,
        text: 0xe6e6e6,
        accent: 0x5865f2,
        other: &[("Danger", 0xc3423f), ("Success", 0x12664f)],
    },
    Bundled {
        name: "Dracula",
        background: 0x282a36,
        text: 0xf8f8f2,
        accent: 0xbd93f9,
        other: &[("Danger", 0xff5555), ("Success", 0x50fa7b)],
    },
    Bundled {
        name: "Ferra",
        background: 0x2b292d,
        text: 0xfecdb2,
        accent: 0xd1d1e0,
        other: &[("Danger", 0xe06b75), ("Success", 0xb1b695)],
    },
    Bundled {
        name: "Gruvbox Dark",
        background: 0x282828,
        text: 0xfbf1c7,
        accent: 0x458588,
        other: &[("Danger", 0xcc241d), ("Success", 0x98971a)],
    },
    Bundled {
        name: "Gruvbox Light",
        background: 0xfbf1c7,
        text: 0x282828,
        accent: 0x458588,
        other: &[("Danger", 0xcc241d), ("Success", 0x98971a)],
    },
    Bundled {
        name: "Kanagawa Dragon",
        background: 0x181616,
        text: 0xc5c9c5,
        accent: 0x223249,
        other: &[("Danger", 0xc4746e), ("Success", 0x8a9a7b)],
    },
    Bundled {
        name: "Kanagawa Lotus",
        background: 0xf2ecbc,
        text: 0x545464,
        accent: 0x4d699b,
        other: &[("Danger", 0xc84053), ("Success", 0x6f894e)],
    },
    Bundled {
        name: "Kanagawa Wave",
        background: 0x1f1f28,
        text: 0xdcd7ba,
        accent: 0x7fb4ca,
        other: &[("Danger", 0xc34043), ("Success", 0x76946a)],
    },
    Bundled {
        name: "Light",
        background: 0xffffff,
        text: 0x000000,
        accent: 0x5865f2,
        other: &[("Danger", 0xc3423f), ("Success", 0x12664f)],
    },
    Bundled {
        name: "Moonfly",
        background: 0x080808,
        text: 0xbdbdbd,
        accent: 0x80a0ff,
        other: &[("Danger", 0xff5454), ("Success", 0x8cc85f)],
    },
    Bundled {
        name: "Nightfly",
        background: 0x011627,
        text: 0xbdc1c6,
        accent: 0x82aaff,
        other: &[("Danger", 0xfc514e), ("Success", 0xa1cd5e)],
    },
    Bundled {
        name: "Nord",
        background: 0x2e3440,
        text: 0xeceff4,
        accent: 0x8fbcbb,
        other: &[("Danger", 0xbf616a), ("Success", 0xa3be8c)],
    },
    Bundled {
        name: "Oxocarbon",
        background: 0x232323,
        text: 0xd0d0d0,
        accent: 0x00b4ff,
        other: &[("Danger", 0xf62d0f), ("Success", 0x00c15a)],
    },
    Bundled {
        name: "Solarized Dark",
        background: 0x002b36,
        text: 0x839496,
        accent: 0x2aa198,
        other: &[("Danger", 0xdc322f), ("Success", 0x859900)],
    },
    Bundled {
        name: "Solarized Light",
        background: 0xfdf6e3,
        text: 0x657b83,
        accent: 0x2aa198,
        other: &[("Danger", 0xdc322f), ("Success", 0x859900)],
    },
    Bundled {
        name: "Tokyo Night",
        background: 0x1a1b26,
        text: 0x9aa5ce,
        accent: 0x2ac3de,
        other: &[("Danger", 0xf7768e), ("Success", 0x9ece6a)],
    },
    Bundled {
        name: "Tokyo Night Light",
        background: 0xd5d6db,
        text: 0x565a6e,
        accent: 0x166775,
        other: &[("Danger", 0x8c4351), ("Success", 0x485e30)],
    },
    Bundled {
        name: "Tokyo Night Storm",
        background: 0x24283b,
        text: 0x9aa5ce,
        accent: 0x2ac3de,
        other: &[("Danger", 0xf7768e), ("Success", 0x9ece6a)],
    },
];
