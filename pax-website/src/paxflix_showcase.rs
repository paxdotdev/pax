use crate::{DeviceFrame, SiteTheme};
use pax_kit::*;
use paxflix::Paxflix;

/// A full-width device fan: one live desktop tablet, linked captures on mobile.
#[pax]
#[file("paxflix_showcase.pax")]
pub struct PaxflixShowcase {
    pub compact: Property<bool>,
    pub stage_scale: Property<f64>,
    pub scene_progress: Property<f64>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_the_desktop_center_is_live_and_all_layouts_keep_three_devices() {
        let template = include_str!("paxflix_showcase.pax");
        assert_eq!(template.matches("<Paxflix ").count(), 1);
        assert!(
            template.contains("embedded=true disable_keyboard=true disable_card_entrances=true")
        );
        assert_eq!(template.matches("url=\"/paxflix/\"").count(), 4);
        assert_eq!(template.matches("id=front_tablet").count(), 1);
        assert_eq!(template.matches("id=phone_wing").count(), 1);
        assert_eq!(template.matches("id=desktop_wing").count(), 1);
        assert!(template.contains("if self.compact"));
        assert!(template.contains("#desktop_wing"));
        assert!(template.contains("#phone_wing"));
        assert!(template.contains("#front_tablet"));
    }

    #[test]
    fn center_tablet_stays_upright_while_capture_wings_keep_their_choreography() {
        let template = include_str!("paxflix_showcase.pax");
        let tablet = template.split("<Group id=front_tablet").nth(1).unwrap();
        assert!(tablet.split('>').next().unwrap().contains("rotate=0deg"));
        let arrival = template.split("#front_tablet {").nth(1).unwrap();
        let tablet_tracks = arrival.split("#desktop_wing").next().unwrap();
        assert!(tablet_tracks.contains("y:"));
        assert!(!tablet_tracks.contains("rotate:"));
        assert_eq!(arrival.matches("rotate:").count(), 2);
    }
}
