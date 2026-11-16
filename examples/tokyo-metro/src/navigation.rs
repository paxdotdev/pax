use crate::AtlasStore;
use pax_kit::*;

const SLUGS: [&str; 14] = [
    "ginza",
    "marunouchi",
    "hibiya",
    "tozai",
    "chiyoda",
    "yurakucho",
    "hanzomon",
    "namboku",
    "fukutoshin",
    "asakusa",
    "mita",
    "shinjuku",
    "oedo",
    "yamanote",
];

pub fn line_path(selected: u32) -> String {
    selected
        .checked_sub(1)
        .and_then(|i| SLUGS.get(i as usize))
        .map(|slug| format!("/line/{slug}"))
        .unwrap_or_else(|| "/".into())
}

fn selection(slug: &str) -> u32 {
    SLUGS
        .iter()
        .position(|s| *s == slug)
        .map_or(0, |i| i as u32 + 1)
}

// Router owns the location, while the atlas stays mounted across branches.
// Only route changes publish into this inbox; they must not push history again.
#[derive(Default)]
pub struct Navigation {
    incoming: Option<u32>,
}

impl Navigation {
    pub fn resolve(&mut self, requested: u32, displayed: u32) -> (u32, Option<String>) {
        if let Some(selected) = self.incoming.take() {
            (selected, None)
        } else {
            (
                requested,
                (requested != displayed).then(|| line_path(requested)),
            )
        }
    }
}

#[pax]
#[inlined(
    <Group width=0px height=0px _raycastable=false/>
    @settings { @mount: mount }
)]
pub struct AtlasRoute {
    pub slug: Property<String>,
}

impl AtlasRoute {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let publish = |ctx: &NodeContext, slug: &str| {
            ctx.with_store(|s: &mut AtlasStore| {
                s.navigation.incoming = Some(selection(slug));
            })
            .expect("atlas route needs the atlas store");
        };
        publish(ctx, &self.slug.get());
        let slug = self.slug.local();
        let deps = [slug.untyped()];
        let ctx = ctx.clone();
        ctx.clone()
            .subscribe(&deps, move || publish(&ctx, &slug.get()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_line_has_a_stable_roundtrip_and_unknown_slugs_return_to_the_map() {
        let network = crate::network::Network::tokyo();
        for (i, line) in network.lines.iter().enumerate() {
            assert_eq!(SLUGS[i], line.name.to_ascii_lowercase());
            assert_eq!(selection(SLUGS[i]), i as u32 + 1);
            assert_eq!(line_path(i as u32 + 1), format!("/line/{}", SLUGS[i]));
        }
        assert_eq!(selection("not-a-line"), 0);
        assert_eq!(line_path(0), "/");
    }

    #[test]
    fn history_changes_and_navigation_echoes_never_push_another_entry() {
        let mut nav = Navigation::default();
        assert_eq!(nav.resolve(14, 0), (14, Some("/line/yamanote".into())));
        nav.incoming = Some(14);
        assert_eq!(nav.resolve(14, 14), (14, None));
        nav.incoming = Some(0);
        assert_eq!(nav.resolve(14, 14), (0, None));
        nav.incoming = Some(14);
        assert_eq!(nav.resolve(0, 0), (14, None));
        assert_eq!(nav.resolve(14, 14), (14, None));
    }
}
