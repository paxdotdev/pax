use crate::SiteTheme;
use pax_kit::*;

#[pax]
#[file("nav_logo.pax")]
pub struct NavLogo {}

#[cfg(test)]
mod tests {
    #[test]
    fn fallback_nav_is_static_and_homepage_portal_twins_stay_mounted() {
        // Mounting the animated logo's clips at the scroll threshold forces a
        // full-page coverage rebuild, even when its playhead is held at one.
        let nav = include_str!("nav_logo.pax");
        assert!(nav.contains("<StaticPaxLogo"));
        assert!(!nav.contains("<AnimatedPaxLogo"));
        let hero = include_str!("hero_section.pax");
        assert!(hero.contains("<SiteLogo id=hero_logo"));
        assert!(hero.contains("@click=self.replay_logo"));
        let shell = include_str!("site_shell.pax");
        assert!(shell.contains("<SiteLogo id=nav_portal_logo"));
        assert!(shell.contains("progress=1 click_to_replay=false"));
        assert!(!shell.contains("if self.nav_logo_entry"));
        let logo = include_str!("site_logo.pax");
        assert!(logo.contains("<AnimatedPaxLogo"));
        assert!(logo.contains("<Radiance"));
        assert!(logo.contains("click_to_replay={self.click_to_replay}"));
    }
}
