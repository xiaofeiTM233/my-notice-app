use crate::model::{Cat, Notif};
use crate::Row;
use chrono::{Datelike, Local, TimeZone, Timelike};

/// 把历史记录按筛选条件展平成列表行，分组标题以 head=true 的行插入
pub fn rows(items: &[Notif], filter: i32, q: &str) -> Vec<Row> {
    let cat = Cat::from_filter(filter);
    let today = day(crate::model::now());
    let mut out: Vec<Row> = Vec::new();
    let mut cur = i32::MIN;

    for n in items {
        if filter == 1 && n.read {
            continue;
        }
        if let Some(c) = cat {
            if n.cat != c {
                continue;
            }
        }
        if !n.hit(q) {
            continue;
        }

        let d = day(n.ts);
        let g = today - d;
        if d != cur {
            cur = d;
            out.push(head(match g {
                0 => "今天".into(),
                1 => "昨天".into(),
                _ => label(n.ts),
            }));
        }
        out.push(Row {
            id: n.id.as_str().into(),
            title: n.title.as_str().into(),
            body: n.body.as_str().into(),
            src: n.src.as_str().into(),
            time: clock(n.ts, g).into(),
            mark: n.cat.mark().into(),
            tag: n.cat.label().into(),
            cat: n.cat.idx(),
            prio: n.prio.idx(),
            read: n.read,
            head: false,
            act: n.act.is_some(),
        });
    }
    out
}

fn head(t: String) -> Row {
    Row {
        id: "".into(),
        title: t.into(),
        body: "".into(),
        src: "".into(),
        time: "".into(),
        mark: "".into(),
        tag: "".into(),
        cat: 0,
        prio: 0,
        read: true,
        head: true,
        act: false,
    }
}

fn day(ts: i64) -> i32 {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.date_naive().num_days_from_ce())
        .unwrap_or(0)
}

fn label(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| format!("{}月{}日", d.month(), d.day()))
        .unwrap_or_else(|| "更早".into())
}

/// 当天显示相对时间，其余显示时刻
fn clock(ts: i64, gap: i32) -> String {
    let Some(d) = Local.timestamp_opt(ts, 0).single() else {
        return String::new();
    };
    if gap == 0 {
        let diff = crate::model::now() - ts;
        if diff < 60 {
            return "刚刚".into();
        }
        if diff < 3600 {
            return format!("{} 分钟前", diff / 60);
        }
    }
    format!("{:02}:{:02}", d.hour(), d.minute())
}
