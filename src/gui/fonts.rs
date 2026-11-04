use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily, FontTweak, ScrollArea, Ui};

const SANS: &str = "MiSans VF";
const SERIF: &str = "Noto Serif SC";
const MONO: &str = "Cascadia Mono";
const ICONS: &str = "Rizum Lucide";

pub(super) fn serif_family() -> FontFamily {
    FontFamily::Name(SERIF.into())
}

pub(super) fn definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    for (name, bytes, weight) in [
        (
            SANS,
            include_bytes!("../../assets/fonts/MiSansVF.ttf").as_slice(),
            400.0,
        ),
        (
            SERIF,
            include_bytes!("../../assets/fonts/NotoSerifSC-VF.ttf").as_slice(),
            700.0,
        ),
        (
            MONO,
            include_bytes!("../../assets/fonts/CascadiaMono.ttf").as_slice(),
            400.0,
        ),
    ] {
        let mut tweak = FontTweak::default();
        tweak.coords.push(b"wght", weight);
        fonts.font_data.insert(
            name.to_owned(),
            Arc::new(FontData::from_static(bytes).tweak(tweak)),
        );
    }
    fonts.font_data.insert(
        ICONS.to_owned(),
        Arc::new(FontData::from_static(lucide_icons::LUCIDE_FONT_BYTES)),
    );

    // MiSans covers CJK; egui's existing symbol and emoji fallbacks remain available.
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, SANS.to_owned());
    proportional.push(ICONS.to_owned());
    let mut serif = proportional.clone();
    serif.insert(0, SERIF.to_owned());
    fonts.families.insert(serif_family(), serif);

    let monospace = fonts.families.entry(FontFamily::Monospace).or_default();
    monospace.splice(0..0, [MONO.to_owned(), SANS.to_owned()]);
    fonts
}

pub(super) fn licenses_ui(ui: &mut Ui) {
    ui.collapsing("Fonts and licenses", |ui| {
        ui.label("Rizum Silicate uses MiSans fonts by Xiaomi, Noto Serif SC by Google, and Cascadia Mono by Microsoft.");
        for (name, license) in [
            (SANS, include_str!("../../assets/fonts/licenses/LICENSE-MiSans.txt")),
            (SERIF, include_str!("../../assets/fonts/licenses/LICENSE-NotoSerifSC.txt")),
            (MONO, include_str!("../../assets/fonts/licenses/LICENSE-CascadiaMono.txt")),
        ] {
            ui.collapsing(name, |ui| {
                ScrollArea::vertical().id_salt(name).max_height(220.0).show(ui, |ui| {
                    ui.add(egui::Label::new(license).wrap().selectable(true));
                });
            });
        }
    });
}
