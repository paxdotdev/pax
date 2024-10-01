pub fn circular_range(
    start: usize,
    end: usize,
    len: usize,
    full_circle: bool,
) -> impl Iterator<Item = usize> {
    let count = if start <= end {
        end - start + 1
    } else {
        len - start + end + 1
    };
    let adjusted_count = if full_circle && start == end {
        len + 1
    } else {
        count
    };
    (0..adjusted_count).map(move |i| (start + i) % len)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_cases() {
        assert_eq!(
            circular_range(0, 4, 5, false).collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4]
        );
        assert_eq!(
            circular_range(4, 2, 8, false).collect::<Vec<_>>(),
            vec![4, 5, 6, 7, 0, 1, 2]
        );
        assert_eq!(circular_range(2, 2, 3, false).collect::<Vec<_>>(), vec![2]);
    }

    #[test]
    fn test_full_circle_cases() {
        assert_eq!(
            circular_range(0, 0, 5, true).collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4, 0]
        );
        assert_eq!(circular_range(0, 0, 5, false).collect::<Vec<_>>(), vec![0]);
        assert_eq!(
            circular_range(2, 1, 5, true).collect::<Vec<_>>(),
            vec![2, 3, 4, 0, 1]
        );
        assert_eq!(
            circular_range(1, 2, 5, true).collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn test_edge_cases() {
        assert_eq!(circular_range(0, 0, 1, false).collect::<Vec<_>>(), vec![0]);
        assert_eq!(
            circular_range(0, 0, 1, true).collect::<Vec<_>>(),
            vec![0, 0]
        );
        assert_eq!(
            circular_range(0, 4, 5, false).collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4]
        );
        assert_eq!(
            circular_range(1, 0, 5, false).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 0]
        );
    }
}
