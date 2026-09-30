use crate::model::{Group, Layout};

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
pub fn positions(g: &Group, visible: &[bool]) -> Vec<Option<Rect>> {
    let indices: Vec<_> = g
        .members
        .iter()
        .enumerate()
        .filter(|(i, _)| g.preserve_spaces || visible.get(*i).copied().unwrap_or(false))
        .map(|(i, _)| i)
        .collect();
    let mut out = vec![None; g.members.len()];
    if g.layout == Layout::Free {
        for i in indices {
            let m = &g.members[i];
            out[i] = Some(Rect {
                x: m.x,
                y: m.y,
                w: m.size,
                h: m.size,
            });
        }
        return out;
    }
    if indices.is_empty() {
        return out;
    }
    let n = indices.len();
    let (cols, rows) = match g.layout {
        Layout::Row => (n, 1),
        Layout::Column => (1, n),
        Layout::Grid => {
            let c = (g.columns as usize).max(1).min(n);
            (c, n.div_ceil(c))
        }
        Layout::Free => unreachable!(),
    };
    let pad = 20.0;
    let gap = g
        .gap
        .min((g.width as f32 - 2.0 * pad).max(0.0) / cols.max(1) as f32)
        .min((g.height as f32 - 2.0 * pad).max(0.0) / rows.max(1) as f32);
    let cell_w = ((g.width as f32 - 2.0 * pad - gap * (cols - 1) as f32) / cols as f32).max(1.0);
    let cell_h = ((g.height as f32 - 2.0 * pad - gap * (rows - 1) as f32) / rows as f32).max(1.0);
    for (slot, i) in indices.into_iter().enumerate() {
        let size = g.members[i]
            .size
            .min(cell_w)
            .min(cell_h - (if g.labels { 24.0 } else { 0.0 }))
            .max(1.0);
        out[i] = Some(Rect {
            x: pad + (slot % cols) as f32 * (cell_w + gap) + (cell_w - size) / 2.0,
            y: pad + (slot / cols) as f32 * (cell_h + gap) + (cell_h - size) / 2.0,
            w: size,
            h: size,
        });
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Member;
    #[test]
    fn reflow_and_preserve_slots_are_distinct() {
        let mut g = Group::new("test");
        g.members = (0..3)
            .map(|i| Member {
                user: i.to_string(),
                ..Default::default()
            })
            .collect();
        let all = positions(&g, &[true, true, true]);
        let compressed = positions(&g, &[true, false, true]);
        assert!(compressed[1].is_none());
        assert_ne!(all[2].unwrap().x, compressed[2].unwrap().x);
        g.preserve_spaces = true;
        assert_eq!(
            positions(&g, &[true, false, true])[2].unwrap().x,
            all[2].unwrap().x
        );
    }
    #[test]
    fn large_groups_have_no_count_cap_and_finite_geometry() {
        let mut g = Group::new("test");
        g.layout = Layout::Grid;
        g.columns = 12;
        g.members = (0..200)
            .map(|i| Member {
                user: i.to_string(),
                ..Default::default()
            })
            .collect();
        let p = positions(&g, &[true; 200]);
        assert_eq!(p.iter().flatten().count(), 200);
        assert!(
            p.iter()
                .flatten()
                .all(|r| r.x.is_finite() && r.y.is_finite() && r.w > 0.0)
        );
    }
}
