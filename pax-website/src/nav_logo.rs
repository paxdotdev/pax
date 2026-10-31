use crate::SiteTheme;
use pax_kit::*;

#[pax]
#[file("nav_logo.pax")]
pub struct NavLogo {}

#[cfg(test)]
mod tests {
    #[test]
    fn fallback_nav_uses_settled_geometry_while_homepage_transfers_one_logo() {
        // Mounting the animated logo's clips at the scroll threshold forces a
        // full-page coverage rebuild, even when its playhead is held at one.
        let nav = include_str!("nav_logo.pax");
        assert!(nav.contains("<StaticPaxLogo"));
        assert!(!nav.contains("<AnimatedPaxLogo"));
        let hero = include_str!("hero_section.pax");
        assert!(hero.contains("<SiteLogo id=hero_logo"));
        let logo = include_str!("site_logo.pax");
        assert!(logo.contains("<AnimatedPaxLogo"));
        assert!(logo.contains("click_to_replay={self.click_to_replay}"));
    }
}
