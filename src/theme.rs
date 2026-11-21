//! Canonical theme model (US-025 dark/light, US-026 custom color scheme).
//!
//! The palette lives here, in Rust, so the TUI and both Flutter transports
//! render the *same* colours. Frontends only receive the resolved snapshot
//! ([`AppTheme`]) and the available swatches ([`AccentOption`]); they never
//! duplicate colour values or guess which accent names are valid.
//!
//! Persistence is owned by [`crate::settings`] — this module only validates
//! names and maps them to concrete colours. An [`Accent`] is a validated newtype
//! so an unknown name cannot reach a renderer.

use serde::Serialize;

/// Canonical accent swatches: `(name, ARGB)`. This table is the single source
/// of truth for both the Flutter seed colour and the TUI accent colour.
const ACCENTS: [(&str, u32); 6] = [
    ("teal", 0xff2f_5d50),
    ("blue", 0xff2f_5d9e),
    ("violet", 0xff6a_3fa0),
    ("rose", 0xffa0_3050),
    ("amber", 0xff9a_6b12),
    ("green", 0xff2f_7a3c),
];

/// Default accent (the original Flutter seed colour).
const DEFAULT_ACCENT: u32 = 0xff2f_5d50;

/// A validated accent. Holds an index into the canonical accent table, so an
/// invalid name can never be represented and [`Accent::next`] is a simple
/// wrapping increment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Accent(u8);

impl Accent {
    /// The original teal accent.
    pub const TEAL: Self = Self(0);
    /// Every accent in display order (used by the TUI cycle shortcut).
    pub const ALL: [Self; 6] = [Self(0), Self(1), Self(2), Self(3), Self(4), Self(5)];

    /// Next accent in [`Accent::ALL`] (wraps around).
    #[must_use]
    pub const fn next(self) -> Self {
        match self.0 {
            0 => Self(1),
            1 => Self(2),
            2 => Self(3),
            3 => Self(4),
            4 => Self(5),
            _ => Self(0),
        }
    }

    /// Validate a persisted or frontend-supplied accent name.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        ACCENTS
            .iter()
            .position(|(n, _)| *n == name)
            .and_then(|i| u8::try_from(i).ok())
            .map(Self)
    }

    /// The stable storage / wire name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        ACCENTS
            .get(usize::from(self.0))
            .or_else(|| ACCENTS.first())
            .map_or("teal", |(name, _)| *name)
    }

    /// Canonical ARGB value, used as Flutter's seed colour.
    #[must_use]
    pub fn argb(self) -> u32 {
        ACCENTS
            .get(usize::from(self.0))
            .map_or(DEFAULT_ACCENT, |(_, argb)| *argb)
    }

    /// RGB bytes for ratatui (the alpha channel is ignored by terminals).
    #[must_use]
    pub fn rgb(self) -> (u8, u8, u8) {
        let argb = self.argb();
        (
            u8::try_from((argb >> 16) & 0xff).unwrap_or_default(),
            u8::try_from((argb >> 8) & 0xff).unwrap_or_default(),
            u8::try_from(argb & 0xff).unwrap_or_default(),
        )
    }
}

/// One selectable swatch, handed to Flutter for its accent picker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccentOption {
    /// Stable accent name, passed back to [`set`].
    pub name: String,
    /// Canonical ARGB value for the swatch.
    pub argb: u32,
}

/// Resolved theme snapshot, returned to both frontends in one round trip.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTheme {
    /// Whether the dark palette is active.
    pub dark: bool,
    /// Name of the active accent.
    pub accent_name: String,
    /// Canonical ARGB of the active accent (Flutter seed colour).
    pub accent_argb: u32,
    /// Every selectable accent swatch.
    pub accents: Vec<AccentOption>,
}

/// Build the current snapshot from persisted settings.
#[must_use]
pub fn current() -> AppTheme {
    let accent = crate::settings::theme_accent();
    AppTheme {
        dark: crate::settings::theme_dark(),
        accent_name: accent.as_str().to_string(),
        accent_argb: accent.argb(),
        accents: ACCENTS
            .iter()
            .map(|(name, argb)| AccentOption {
                name: (*name).to_string(),
                argb: *argb,
            })
            .collect(),
    }
}

/// Persist a theme choice. The accent name is validated *before* any write so
/// a bogus frontend value cannot leave the two settings out of sync.
///
/// # Errors
/// Returns an error for an unknown accent name or a failed database write.
pub fn set(dark: bool, accent_name: &str) -> Result<(), String> {
    let accent =
        Accent::parse(accent_name).ok_or_else(|| format!("unknown accent color: {accent_name}"))?;
    crate::settings::set_theme_dark(dark)?;
    crate::settings::set_theme_accent(accent)
}

/// Concrete ratatui colours for the active theme.
///
/// Kept behind the `tui` feature so the library builds without ratatui (the
/// mobile/web builds do not need it).
#[cfg(feature = "tui")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// Dark variant vs light variant.
    pub dark: bool,
    /// Validated accent colour.
    pub accent: Accent,
}

#[cfg(feature = "tui")]
impl Default for Palette {
    fn default() -> Self {
        Self {
            dark: true,
            accent: Accent::TEAL,
        }
    }
}

#[cfg(feature = "tui")]
impl Palette {
    /// Snapshot the persisted theme once, e.g. when the TUI state is built.
    #[must_use]
    pub fn current() -> Self {
        Self {
            dark: crate::settings::theme_dark(),
            accent: crate::settings::theme_accent(),
        }
    }

    /// Accent colour (tabs, unread banner).
    #[must_use]
    pub fn accent(self) -> ratatui::style::Color {
        let (r, g, b) = self.accent.rgb();
        ratatui::style::Color::Rgb(r, g, b)
    }

    /// Background for highlighted rows.
    #[must_use]
    pub const fn selection_bg(self) -> ratatui::style::Color {
        if self.dark {
            ratatui::style::Color::DarkGray
        } else {
            ratatui::style::Color::Rgb(210, 210, 210)
        }
    }

    /// Background for modal popups.
    #[must_use]
    pub const fn popup_bg(self) -> ratatui::style::Color {
        if self.dark {
            ratatui::style::Color::Black
        } else {
            ratatui::style::Color::White
        }
    }
}

#[cfg(all(test, feature = "tui"))]
mod tests {
    use super::*;

    #[test]
    fn accent_names_are_validated_and_have_canonical_values() {
        assert_eq!(Accent::parse("teal"), Some(Accent::TEAL));
        assert_eq!(Accent::TEAL.argb(), 0xff2f_5d50);
        assert_eq!(Accent::TEAL.rgb(), (0x2f, 0x5d, 0x50));
        assert_eq!(Accent::parse("TEAL"), None);
        assert_eq!(Accent::parse("nope"), None);
    }

    #[test]
    fn palette_selection_contrasts_with_its_background() {
        let dark = Palette {
            dark: true,
            accent: Accent::TEAL,
        };
        let light = Palette {
            dark: false,
            accent: Accent::TEAL,
        };
        assert_ne!(dark.selection_bg(), light.selection_bg());
        assert_ne!(dark.popup_bg(), light.popup_bg());
    }
}
