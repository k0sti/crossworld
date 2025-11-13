use cube::{Cube, CubeCoord};
use std::collections::{HashMap, VecDeque};
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub pos: [i32; 3],
    pub depth: u32,
}

impl From<CubeCoord> for CacheKey {
    fn from(coord: CubeCoord) -> Self {
        Self {
            pos: [coord.pos.x, coord.pos.y, coord.pos.z],
            depth: coord.depth,
        }
    }
}

#[derive(Debug)]
struct CacheInner {
    map: HashMap<CacheKey, Cube<u8>>,
    order: VecDeque<CacheKey>,
}

/// Lightweight LRU cache for cube regions.
#[derive(Debug)]
pub struct WorldCache {
    capacity: usize,
    inner: Mutex<CacheInner>,
}

impl WorldCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            inner: Mutex::new(CacheInner {
                map: HashMap::new(),
                order: VecDeque::new(),
            }),
        }
    }

    pub async fn get(&self, coord: CubeCoord) -> Option<Cube<u8>> {
        if self.capacity == 0 {
            return None;
        }

        let mut inner = self.inner.lock().await;
        let key = CacheKey::from(coord);
        if let Some(cube) = inner.map.get(&key).cloned() {
            inner.order.retain(|existing| existing != &key);
            inner.order.push_back(key);
            return Some(cube);
        }
        None
    }

    pub async fn insert(&self, coord: CubeCoord, cube: Cube<u8>) {
        if self.capacity == 0 {
            return;
        }

        let mut inner = self.inner.lock().await;
        let key = CacheKey::from(coord);

        if inner.map.contains_key(&key) {
            inner.map.insert(key.clone(), cube);
            inner.order.retain(|existing| existing != &key);
            inner.order.push_back(key);
            return;
        }

        if inner.map.len() >= self.capacity {
            if let Some(oldest) = inner.order.pop_front() {
                inner.map.remove(&oldest);
            }
        }

        inner.order.push_back(key.clone());
        inner.map.insert(key, cube);
    }

    pub async fn clear(&self) {
        let mut inner = self.inner.lock().await;
        inner.map.clear();
        inner.order.clear();
    }
}
