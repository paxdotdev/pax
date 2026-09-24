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
    let mut shelves: Vec<Shelf> = [
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
    .collect();

    // Keep the original browsing order, then add collections with deliberate
    // selections rather than repeating another rotation of the same ten cards.
    for (title, film_ids) in [
        ("Worlds beyond ours", [0, 3, 8, 18, 20, 23, 27, 30, 32, 11]),
        (
            "Love and other detours",
            [6, 13, 21, 29, 33, 4, 2, 9, 16, 26],
        ),
        ("Keep you guessing", [1, 7, 11, 14, 18, 27, 28, 30, 33, 0]),
        ("The great wide open", [5, 15, 17, 19, 24, 26, 34, 3, 9, 32]),
        ("One more chance", [12, 22, 25, 4, 16, 5, 15, 10, 21, 31]),
        ("Together, somehow", [2, 6, 9, 10, 13, 21, 26, 29, 31, 32]),
    ] {
        shelves.push(Shelf {
            id: shelves.len(),
            title: title.into(),
            movies: film_ids
                .iter()
                .map(|id| {
                    movies
                        .iter()
                        .find(|movie| movie.id == *id)
                        .expect("curated film belongs to the catalog")
                        .clone()
                })
                .collect(),
        });
    }
    shelves
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
    fn shelves_preserve_identity_and_include_every_film() {
        let movies = catalog();
        let rows = shelves(&movies);
        let mut counts = vec![0; movies.len()];
        assert_eq!(rows.len(), 16);
        assert_eq!(
            rows.iter()
                .map(|row| &row.title)
                .collect::<HashSet<_>>()
                .len(),
            16
        );
        for (id, row) in rows.iter().enumerate() {
            assert_eq!(row.id, id);
            assert_eq!(row.movies.len(), 10);
            assert_eq!(
                row.movies
                    .iter()
                    .map(|m| m.id)
                    .collect::<HashSet<_>>()
                    .len(),
                10
            );
            for movie in &row.movies {
                counts[movie.id] += 1;
                let original = &movies[movie.id];
                assert_eq!(movie.title, original.title);
                assert_eq!(movie.still, original.still);
                assert_eq!(movie.thumbnail, original.thumbnail);
                assert_eq!(movie.synopsis, original.synopsis);
            }
        }
        assert_eq!(counts.iter().sum::<usize>(), 160);
        assert!(counts.iter().all(|count| *count >= 2));
        for (index, row) in rows.iter().enumerate().skip(10) {
            let selection: HashSet<_> = row.movies.iter().map(|movie| movie.id).collect();
            assert!(
                rows[..index].iter().all(|other| {
                    other
                        .movies
                        .iter()
                        .map(|movie| movie.id)
                        .collect::<HashSet<_>>()
                        != selection
                }),
                "new collection repeats an existing selection: {}",
                row.title
            );
        }
    }
}
