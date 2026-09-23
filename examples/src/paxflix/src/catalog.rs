use pax_kit::*;

#[pax]
pub struct Movie {
    pub id: usize,
    pub slug: String,
    pub title: String,
    pub year: String,
    pub runtime: String,
    pub rating: String,
    pub genres: String,
    pub synopsis: String,
    pub still: String,
    pub thumbnail: String,
}

#[pax]
pub struct Shelf {
    pub id: usize,
    pub title: String,
    pub movies: Vec<Movie>,
}

pub fn catalog() -> Vec<Movie> {
    serde_json::from_str(include_str!("../catalog.json")).expect("checked-in film catalog is valid")
}

pub fn shelves(movies: &[Movie]) -> Vec<Shelf> {
    [
        "Your next great watch",
        "New to Paxflix",
        "Stories that stay with you",
        "Escape the everyday",
        "A different point of view",
        "The weekend edit",
        "Go somewhere new",
        "After hours",
        "Unexpected connections",
        "Worth another look",
    ]
    .iter()
    .enumerate()
    .map(|(id, title)| Shelf {
        id,
        title: (*title).into(),
        // All placements are real nodes; films repeat evenly without reshuffling.
        movies: (0..10)
            .map(|col| movies[(id * 10 + col) % movies.len()].clone())
            .collect(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_assets_and_identities_are_complete() {
        let movies = catalog();
        assert_eq!(movies.len(), 35);
        assert_eq!(
            movies.iter().map(|m| &m.slug).collect::<HashSet<_>>().len(),
            35
        );
        for (id, movie) in movies.iter().enumerate() {
            assert_eq!(movie.id, id);
            assert!(!movie.title.is_empty() && !movie.synopsis.is_empty());
            for asset in [&movie.still, &movie.thumbnail] {
                assert!(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join(asset)
                        .is_file(),
                    "{asset}"
                );
            }
        }
    }

    #[test]
    fn shelves_preserve_identity_and_distribute_every_film_evenly() {
        let movies = catalog();
        let rows = shelves(&movies);
        let mut counts = vec![0; movies.len()];
        assert_eq!(rows.len(), 10);
        for row in rows {
            assert_eq!(row.movies.len(), 10);
            assert_eq!(
                row.movies
                    .iter()
                    .map(|m| m.id)
                    .collect::<HashSet<_>>()
                    .len(),
                10
            );
            for movie in row.movies {
                counts[movie.id] += 1;
                let original = &movies[movie.id];
                assert_eq!(movie.title, original.title);
                assert_eq!(movie.still, original.still);
                assert_eq!(movie.thumbnail, original.thumbnail);
                assert_eq!(movie.synopsis, original.synopsis);
            }
        }
        assert_eq!(counts.iter().sum::<usize>(), 100);
        assert!(counts.iter().all(|count| (2..=3).contains(count)));
    }
}
