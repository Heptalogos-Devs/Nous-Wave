use std::collections::{BTreeMap, VecDeque};

/// A shortest connectivity witness in one bounded observation, not attribution
/// of accumulated score. Every visited node is queued at most once.
pub(crate) fn activated_routes<K: Copy + Ord>(
    seeds: impl IntoIterator<Item = K>,
    edges: impl IntoIterator<Item = (K, K)>,
    max_hops: usize,
) -> BTreeMap<K, Vec<K>> {
    let mut outgoing = BTreeMap::<K, Vec<K>>::new();
    for (from, to) in edges {
        outgoing.entry(from).or_default().push(to);
    }
    for targets in outgoing.values_mut() {
        targets.sort_unstable();
        targets.dedup();
    }
    let mut routes = BTreeMap::new();
    let mut queue = VecDeque::new();
    for seed in seeds {
        if let std::collections::btree_map::Entry::Vacant(entry) = routes.entry(seed) {
            entry.insert(vec![seed]);
            queue.push_back(seed);
        }
    }
    while let Some(node) = queue.pop_front() {
        let path = routes[&node].clone();
        if path.len() > max_hops.min(32) {
            continue;
        }
        for target in outgoing.get(&node).into_iter().flatten() {
            if let std::collections::btree_map::Entry::Vacant(entry) = routes.entry(*target) {
                let mut next = path.clone();
                next.push(*target);
                entry.insert(next);
                queue.push_back(*target);
            }
        }
    }
    routes
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn witnesses_only_reachable_bounded_paths_and_terminates_on_cycles() {
        let paths = activated_routes([1, 5], [(1, 2), (2, 1), (2, 3), (3, 4), (5, 4), (8, 9)], 2);
        assert_eq!(paths[&3], [1, 2, 3]);
        assert_eq!(paths[&4], [5, 4]);
        assert!(!paths.contains_key(&8));
        assert!(!activated_routes([1], [(1, 2), (2, 3)], 1).contains_key(&3));
    }
}
