// 程序化生成托盘/窗口图标，省去打包位图资源

const ACCENT: [f32; 3] = [0.184, 0.435, 0.922];
const BADGE: [f32; 3] = [0.937, 0.267, 0.267];

pub fn rgba(size: u32, badge: bool) -> Vec<u8> {
    let s = size as f32;
    let mut out = vec![0u8; (size * size * 4) as usize];
    let ss = 3; // 3x3 超采样做抗锯齿

    for y in 0..size {
        for x in 0..size {
            let (mut bg, mut fg, mut bd) = (0.0f32, 0.0f32, 0.0f32);
            for sy in 0..ss {
                for sx in 0..ss {
                    let px = x as f32 + (sx as f32 + 0.5) / ss as f32;
                    let py = y as f32 + (sy as f32 + 0.5) / ss as f32;
                    let (u, v) = (px / s, py / s);
                    if round_box(px, py, s, s * 0.24) {
                        bg += 1.0;
                    }
                    if bell(u, v) {
                        fg += 1.0;
                    }
                    if badge && circle(u, v, 0.78, 0.24, 0.19) {
                        bd += 1.0;
                    }
                }
            }
            let n = (ss * ss) as f32;
            let (bg, fg, bd) = (bg / n, fg / n, bd / n);

            let mut c = [ACCENT[0], ACCENT[1], ACCENT[2], bg];
            c = over(c, [1.0, 1.0, 1.0, fg * bg]);
            c = over(c, [BADGE[0], BADGE[1], BADGE[2], bd]);

            let i = ((y * size + x) * 4) as usize;
            out[i] = (c[0] * 255.0) as u8;
            out[i + 1] = (c[1] * 255.0) as u8;
            out[i + 2] = (c[2] * 255.0) as u8;
            out[i + 3] = (c[3] * 255.0) as u8;
        }
    }
    out
}

fn over(dst: [f32; 4], src: [f32; 4]) -> [f32; 4] {
    let a = src[3] + dst[3] * (1.0 - src[3]);
    if a <= 0.001 {
        return [0.0; 4];
    }
    let m = |i: usize| (src[i] * src[3] + dst[i] * dst[3] * (1.0 - src[3])) / a;
    [m(0), m(1), m(2), a]
}

fn round_box(x: f32, y: f32, s: f32, r: f32) -> bool {
    if x < 0.0 || y < 0.0 || x > s || y > s {
        return false;
    }
    let dx = if x < r {
        r - x
    } else if x > s - r {
        x - (s - r)
    } else {
        0.0
    };
    let dy = if y < r {
        r - y
    } else if y > s - r {
        y - (s - r)
    } else {
        0.0
    };
    dx * dx + dy * dy <= r * r
}

fn circle(x: f32, y: f32, cx: f32, cy: f32, r: f32) -> bool {
    let (dx, dy) = (x - cx, y - cy);
    dx * dx + dy * dy <= r * r
}

fn rect(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
    x >= x0 && x <= x1 && y >= y0 && y <= y1
}

/// 归一化坐标下的铃铛剪影
fn bell(u: f32, v: f32) -> bool {
    circle(u, v, 0.5, 0.24, 0.05)
        || circle(u, v, 0.5, 0.46, 0.19)
        || rect(u, v, 0.31, 0.46, 0.69, 0.60)
        || rect(u, v, 0.25, 0.60, 0.75, 0.655)
        || circle(u, v, 0.5, 0.72, 0.06)
}
