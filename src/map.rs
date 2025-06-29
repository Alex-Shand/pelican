//! Immutable, claimable map

use std::{borrow::Borrow, collections::HashMap, hash::Hash, sync::Arc};

use trivial::{Claim, ClaimArc};

/// Immutable, claimable map
#[derive(Debug)]
pub struct Map<K: Hash + Eq, V>(ClaimArc<MapInner<K, V>>);

#[derive(Debug)]
struct MapInner<K: Hash + Eq, V> {
    parent: Option<Map<K, V>>,
    current: HashMap<K, V>,
}

impl<K: Hash + Eq, V> Claim for Map<K, V> {
    fn claim(&self) -> Self {
        Self(self.0.claim())
    }
}

impl<K: Hash + Eq, V> Default for Map<K, V> {
    fn default() -> Self {
        Self(ClaimArc::new(MapInner {
            parent: None,
            current: HashMap::new(),
        }))
    }
}

impl<K: Hash + Eq, V> Map<K, V> {
    /// Constructor
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Produce a new map with the given key value pair added. If the map
    /// already contains a value for the key it will be replaced
    ///
    /// If the specific map object at runtime is not shared this is an inplace
    /// update, otherwise the result is a new map which holds the new key value
    /// pair and a reference to the previous map
    #[must_use]
    pub fn update(mut self, k: K, v: V) -> Self {
        let Self(ClaimArc(arc)) = &mut self;
        if let Some(inner) = Arc::get_mut(arc) {
            // We have the only reference to the map we can update in place
            let _ = inner.current.insert(k, v);
            return self;
        }
        // Somebody else also has a handle to the map, we have to insert in a
        // new layer
        let mut current = HashMap::new();
        let _ = current.insert(k, v);
        Self(ClaimArc::new(MapInner {
            parent: Some(self),
            current,
        }))
    }

    /// Retrieve a value from the map
    pub fn get(&self, k: impl Borrow<K>) -> Option<&V> {
        if let Some(v) = self.0.current.get(k.borrow()) {
            return Some(v);
        }
        if let Some(parent) = &self.0.parent {
            return parent.get(k);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use trivial::Claim as _;

    use super::Map;

    // No Copy, Claim, Trivial or Clone
    #[derive(Debug, Hash, PartialEq, Eq)]
    struct Singular(usize);

    #[test]
    fn empty_map() {
        let map: Map<Singular, Singular> = Map::new();
        assert!(map.get(Singular(0)).is_none());
    }

    #[test]
    fn single_owner() {
        let map = Map::new()
            .update(Singular(0), Singular(1))
            .update(Singular(2), Singular(3))
            .update(Singular(4), Singular(5));

        assert_eq!(map.get(Singular(0)), Some(&Singular(1)));
        assert_eq!(map.get(Singular(2)), Some(&Singular(3)));
        assert_eq!(map.get(Singular(4)), Some(&Singular(5)));

        let inner = &map.0.current;
        assert_eq!(inner.get(&Singular(0)), Some(&Singular(1)));
        assert_eq!(inner.get(&Singular(2)), Some(&Singular(3)));
        assert_eq!(inner.get(&Singular(4)), Some(&Singular(5)));
    }

    #[test]
    fn layers() {
        let map = Map::new().update(Singular(0), Singular(1));
        let map2 = map.claim().update(Singular(2), Singular(3));
        let map3 = map2.claim().update(Singular(4), Singular(5));

        assert_eq!(map3.get(Singular(0)), Some(&Singular(1)));
        assert_eq!(map3.get(Singular(2)), Some(&Singular(3)));
        assert_eq!(map3.get(Singular(4)), Some(&Singular(5)));

        assert_eq!(map2.get(Singular(0)), Some(&Singular(1)));
        assert_eq!(map2.get(Singular(2)), Some(&Singular(3)));
        assert!(map2.get(Singular(4)).is_none());

        assert_eq!(map.get(Singular(0)), Some(&Singular(1)));
        assert!(map.get(Singular(2)).is_none());
        assert!(map.get(Singular(4)).is_none());

        let inner1 = &map.0.current;
        let inner2 = &map2.0.current;
        let inner3 = &map3.0.current;

        assert_eq!(inner1.get(&Singular(0)), Some(&Singular(1)));
        assert!(inner1.get(&Singular(2)).is_none());
        assert!(inner1.get(&Singular(4)).is_none());

        assert!(inner2.get(&Singular(0)).is_none());
        assert_eq!(inner2.get(&Singular(2)), Some(&Singular(3)));
        assert!(inner2.get(&Singular(4)).is_none());

        assert!(inner3.get(&Singular(0)).is_none());
        assert!(inner3.get(&Singular(2)).is_none());
        assert_eq!(inner3.get(&Singular(4)), Some(&Singular(5)));
    }

    #[test]
    fn shadowing() {
        let map = Map::new().update(Singular(0), Singular(1));
        let map2 = map.claim().update(Singular(0), Singular(2));

        assert_eq!(map.get(Singular(0)), Some(&Singular(1)));
        assert_eq!(map2.get(Singular(0)), Some(&Singular(2)));
    }

    #[test]
    fn branching() {
        let map = Map::new().update(Singular(0), Singular(1));
        let map2 = map.claim().update(Singular(0), Singular(2));
        let map3 = map.claim().update(Singular(3), Singular(4));

        assert_eq!(map.get(Singular(0)), Some(&Singular(1)));
        assert_eq!(map2.get(Singular(0)), Some(&Singular(2)));

        assert!(map.get(Singular(3)).is_none());
        assert!(map2.get(Singular(3)).is_none());

        assert_eq!(map3.get(Singular(0)), Some(&Singular(1)));
        assert_eq!(map3.get(Singular(3)), Some(&Singular(4)));
    }
}
