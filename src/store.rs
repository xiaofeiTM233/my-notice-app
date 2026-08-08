use crate::model::Notif;
use std::path::PathBuf;

/// 历史通知，按时间倒序保存在内存，可选落盘为 JSON
pub struct Store {
    items: Vec<Notif>,
    max: usize,
    persist: bool,
    path: PathBuf,
}

impl Store {
    pub fn load(dir: PathBuf, max: usize, persist: bool) -> Self {
        let path = dir.join("history.json");
        let mut items: Vec<Notif> = if persist {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            vec![]
        };
        items.sort_by(|a, b| b.ts.cmp(&a.ts));
        items.truncate(max);
        Store {
            items,
            max,
            persist,
            path,
        }
    }

    pub fn items(&self) -> &[Notif] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn unread(&self) -> usize {
        self.items.iter().filter(|n| !n.read).count()
    }

    /// 已存在同 id 时返回 false（后端重发时去重）
    pub fn push(&mut self, n: Notif) -> bool {
        if self.items.iter().any(|x| x.id == n.id) {
            return false;
        }
        let at = self
            .items
            .iter()
            .position(|x| x.ts <= n.ts)
            .unwrap_or(self.items.len());
        self.items.insert(at, n);
        self.items.truncate(self.max);
        true
    }

    pub fn mark(&mut self, id: &str) {
        if let Some(n) = self.items.iter_mut().find(|n| n.id == id) {
            n.read = true;
        }
    }

    pub fn mark_all(&mut self) {
        self.items.iter_mut().for_each(|n| n.read = true);
    }

    pub fn del(&mut self, id: &str) {
        self.items.retain(|n| n.id != id);
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn get(&self, id: &str) -> Option<&Notif> {
        self.items.iter().find(|n| n.id == id)
    }

    pub fn save(&self) {
        if !self.persist {
            return;
        }
        if let Some(d) = self.path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Ok(s) = serde_json::to_string(&self.items) {
            let _ = std::fs::write(&self.path, s);
        }
    }
}
