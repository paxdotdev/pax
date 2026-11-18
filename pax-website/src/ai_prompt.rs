use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

pub const INTRO: &str = "Let's build an app with Pax. Read https://www.pax.dev/ai/ to get started.";
pub const OUTRO: &str =
    "Build it and run a local preview. Keep the first version small and polished.";

// Local, credential-free briefs: a first app should not depend on a service signup.
pub const IDEAS: [&str; 25] = [
    "Make a focus timer with adjustable work and break intervals, a playful animated countdown, and start, pause, and reset controls.",
    "Make a habit tracker with a colorful weekly grid. Let me name three habits and tap each day to mark it complete.",
    "Make a color-palette studio. Generate five colors, lock favorites, shuffle the rest, and show each color's hex value.",
    "Make a gradient playground with two editable colors and an angle slider. Show the gradient in a big live preview.",
    "Make a dice roller with a satisfying rolling animation, a choice of one or two dice, and a history of the last ten rolls.",
    "Make a memory-matching game with a small grid of colorful symbol cards, a move counter, and a playful celebration when every pair is found.",
    "Make a tiny sketchbook with a choice of brush colors and sizes, an eraser, and a clear button. Give the drawing canvas most of the screen.",
    "Make a pixel-art toy with a 16 by 16 grid, a small color palette, and a clear button. Let me paint by clicking or dragging across the cells.",
    "Make a breathing companion with a slowly expanding and contracting circle, inhale and exhale cues, and adjustable breathing intervals.",
    "Make a unit converter for length, weight, and temperature. Update the answer as I type and make swapping units feel effortless.",
    "Make a tip calculator with a bill amount, tip percentage, and number of people. Show the total and each person's share clearly.",
    "Make a small task board with To do, Doing, and Done columns. Let me add tasks and move them between columns with simple buttons.",
    "Make a mood journal for the current week. Let me choose a color and a short note for each day, then see the week as a colorful mosaic.",
    "Make a water tracker with a bottle that fills as I add a glass. Let me adjust my daily goal and undo the last glass.",
    "Make a workout interval timer with adjustable activity and rest periods, a round counter, and large controls I can use from across the room.",
    "Make a word-count workspace with a generous text area and live word, character, and sentence counts. Keep the interface calm and distraction-free.",
    "Make a flashcard app with a few sample cards. Let me add a question and answer, flip a card, and mark whether I remembered it.",
    "Make a sorting visualizer with colored bars. Let me shuffle them, step through bubble sort, or play the animation at an adjustable speed.",
    "Make Conway's Game of Life with a clickable grid, play and pause controls, a speed slider, and a few starting patterns.",
    "Make a spring-motion playground with a draggable ball and adjustable stiffness and damping. Show how the ball settles after I release it.",
    "Make a packing checklist with categories for clothes, toiletries, and essentials. Let me add items and see a progress bar as I check them off.",
    "Make a recipe scaler with a short sample ingredient list. Let me change the serving count and see every quantity update immediately.",
    "Make an event countdown with an editable event name and date, big remaining-time digits, and a celebratory animation when the time arrives.",
    "Make a typography playground with editable sample text, font-size and line-height controls, and a few foreground and background color choices.",
    "Make an orbital toy with colorful dots moving around a center. Let me adjust their speed and orbit radius, and pause to explore the composition.",
];

pub fn prompt(idea: &str) -> String {
    format!("{INTRO}\n\n{}\n\n{OUTRO}", idea.trim())
}

pub fn codex_url(idea: &str) -> String {
    // Only draft text is handed off; no guessed workspace and no auto-submit.
    format!(
        "codex://threads/new?prompt={}",
        utf8_percent_encode(&prompt(idea), NON_ALPHANUMERIC)
    )
}

pub fn different_index(current: usize, draw: usize) -> usize {
    // Draw uniformly from 24 slots, then skip the current idea.
    (current + 1 + draw % (IDEAS.len() - 1)) % IDEAS.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use percent_encoding::percent_decode_str;
    use std::collections::HashSet;

    #[test]
    fn twenty_five_distinct_self_contained_ideas() {
        assert_eq!(IDEAS.len(), 25);
        assert_eq!(IDEAS.iter().collect::<HashSet<_>>().len(), 25);
        assert!(IDEAS
            .iter()
            .all(|idea| idea.starts_with("Make ") && idea.ends_with('.')));
    }

    #[test]
    fn shuffle_reaches_every_other_idea_without_repeating() {
        for current in 0..IDEAS.len() {
            let choices: HashSet<_> = (0..24).map(|draw| different_index(current, draw)).collect();
            assert_eq!(choices.len(), 24);
            assert!(!choices.contains(&current));
        }
    }

    #[test]
    fn handoff_round_trips_the_exact_copied_prompt() {
        for idea in IDEAS
            .iter()
            .copied()
            .chain(["  Make a café timer: ☕ + tea & biscuits?\nKeep 100% local.  "])
        {
            let url = codex_url(idea);
            let encoded = url.strip_prefix("codex://threads/new?prompt=").unwrap();
            assert!(!encoded.contains('&'));
            assert!(!encoded.contains('#'));
            assert_eq!(
                percent_decode_str(encoded).decode_utf8().unwrap(),
                prompt(idea)
            );
        }
    }
}
