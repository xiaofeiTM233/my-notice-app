use crate::model::rel_time;
use crate::NItem;
use slint::VecModel;
use std::rc::Rc;

// 通知仓库：全量数据 + 当前筛选视图，view 同步到 Slint 列表模型
pub struct Store {
    all: Vec<NItem>,
    view: Vec<NItem>,
    search: String,
    cat: i32,
    model: Rc<VecModel<NItem>>,
    max: usize,
}

impl Store {
    pub fn new(model: Rc<VecModel<NItem>>) -> Self {
        Self { all: vec![], view: vec![], search: String::new(), cat: -1, model, max: 500 }
    }

    pub fn add(&mut self, it: NItem) {
        if self.all.iter().any(|x| x.id == it.id) {
            return; // 去重
        }
        self.all.insert(0, it);
        if self.all.len() > self.max {
            self.all.pop();
        }
    }

    pub fn mark_read(&mut self, id: &str) {
        for x in self.all.iter_mut() {
            if x.id == id {
                x.read = true;
            }
        }
    }

    pub fn mark_all(&mut self) {
        for x in self.all.iter_mut() {
            x.read = true;
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.all.retain(|x| x.id != id);
    }

    pub fn clear(&mut self) {
        self.all.clear();
    }

    pub fn set_search(&mut self, s: String) {
        self.search = s;
    }

    pub fn set_cat(&mut self, c: i32) {
        self.cat = c;
    }

    pub fn unread(&self) -> i32 {
        self.all.iter().filter(|x| !x.read).count() as i32
    }

    pub fn total(&self) -> i32 {
        self.all.len() as i32
    }

    // 重建视图并刷新模型（顺带更新相对时间）
    pub fn refresh(&mut self) {
        for x in self.all.iter_mut() {
            x.rel = rel_time(x.ts as i64).into();
        }
        let q = self.search.to_lowercase();
        self.view = self
            .all
            .iter()
            .filter(|x| (self.cat < 0 || x.cat == self.cat) && matches_search(x, &q))
            .cloned()
            .collect();
        self.model.set_vec(self.view.clone());
    }
}

fn matches_search(x: &NItem, q: &str) -> bool {
    q.is_empty()
        || x.title.to_lowercase().contains(q)
        || x.body.to_lowercase().contains(q)
}
