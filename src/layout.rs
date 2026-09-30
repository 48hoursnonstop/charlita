use crate::model::{Group, Layout};

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
pub struct Composition {
    pub width: u32,
    pub height: u32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub positions: Vec<Option<Rect>>,
}
pub fn compose(g: &Group, visible: &[bool], jumps: &[f32]) -> Composition {
    let positions = if g.auto_size {
        return automatic(g, visible, jumps);
    } else {
        fixed_positions(g, visible)
    };
    Composition {
        width: g.width,
        height: g.height,
        origin_x: 0.0,
        origin_y: 0.0,
        positions,
    }
}
fn indices(g: &Group, visible: &[bool]) -> Vec<usize> {
    g.members
        .iter()
        .enumerate()
        .filter(|(i, _)| g.preserve_spaces || visible.get(*i).copied().unwrap_or(false))
        .map(|(i, _)| i)
        .collect()
}
fn automatic(g: &Group, visible: &[bool], jumps: &[f32]) -> Composition {
    let pad = 20.0;
    let label = if g.labels { 32.0 } else { 0.0 };
    let jump = |i: usize| jumps.get(i).copied().unwrap_or(10.0);
    let mut result = Composition {
        width: 64,
        height: 64,
        origin_x: 0.0,
        origin_y: 0.0,
        positions: vec![None; g.members.len()],
    };
    let selected = indices(g, visible);
    if g.layout == Layout::Free {
        // Visibility never shifts a free composition's coordinate system.
        if g.members.is_empty() {
            return result;
        }
        let mut left = f32::INFINITY;
        let mut top = f32::INFINITY;
        let mut right = f32::NEG_INFINITY;
        let mut bottom = f32::NEG_INFINITY;
        for (i, member) in g.members.iter().enumerate() {
            left = left.min(member.x);
            top = top.min(member.y - jump(i));
            right = right.max(member.x + member.size);
            bottom = bottom.max(member.y + member.size + label);
        }
        result.origin_x = left - pad;
        result.origin_y = top - pad;
        result.width = (right - left + 2.0 * pad).ceil().max(64.0) as u32;
        result.height = (bottom - top + 2.0 * pad).ceil().max(64.0) as u32;
        for i in selected {
            let m = &g.members[i];
            result.positions[i] = Some(Rect {
                x: m.x - result.origin_x,
                y: m.y - result.origin_y,
                w: m.size,
                h: m.size,
            });
        }
        return result;
    }
    let count = selected.len();
    if count == 0 {
        return result;
    }
    let columns = match g.layout {
        Layout::Row => count,
        Layout::Column => 1,
        Layout::Grid => (g.columns as usize).max(1).min(count),
        Layout::Free => unreachable!(),
    };
    let rows = count.div_ceil(columns);
    let mut widths = vec![0.0f32; columns];
    let mut heights = vec![0.0f32; rows];
    let mut row_jumps = vec![0.0f32; rows];
    for (slot, &i) in selected.iter().enumerate() {
        widths[slot % columns] = widths[slot % columns].max(g.members[i].size);
        heights[slot / columns] = heights[slot / columns].max(g.members[i].size);
        row_jumps[slot / columns] = row_jumps[slot / columns].max(jump(i));
    }
    let mut x_offsets = Vec::with_capacity(columns);
    let mut x = pad;
    for width in &widths {
        x_offsets.push(x);
        x += width + g.gap;
    }
    let mut y_offsets = Vec::with_capacity(rows);
    let mut y = pad;
    for row in 0..rows {
        y_offsets.push(y);
        y += heights[row] + row_jumps[row] + label + g.gap;
    }
    result.width = (x - g.gap + pad).ceil().max(64.0) as u32;
    result.height = (y - g.gap + pad).ceil().max(64.0) as u32;
    for (slot, i) in selected.into_iter().enumerate() {
        let column = slot % columns;
        let row = slot / columns;
        let size = g.members[i].size;
        result.positions[i] = Some(Rect {
            x: x_offsets[column] + (widths[column] - size) / 2.0,
            y: y_offsets[row] + row_jumps[row] + (heights[row] - size) / 2.0,
            w: size,
            h: size,
        });
    }
    result
}
fn fixed_positions(g: &Group, visible: &[bool]) -> Vec<Option<Rect>> {
    let indices = indices(g, visible);
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
    fn positions(g: &Group, visible: &[bool]) -> Vec<Option<Rect>> {
        compose(g, visible, &[]).positions
    }
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
    #[test]
    fn automatic_layout_wraps_content_without_shrinking_guests() {
        let mut g = Group::new("auto");
        assert!(g.auto_size);
        g.members = [100.0, 200.0, 80.0]
            .into_iter()
            .enumerate()
            .map(|(i, size)| Member {
                user: i.to_string(),
                size,
                ..Default::default()
            })
            .collect();
        let jumps = [10.0, 60.0, 0.0];
        for layout in [Layout::Row, Layout::Column, Layout::Grid] {
            g.layout = layout;
            g.columns = 2;
            let result = compose(&g, &[true; 3], &jumps);
            for (i, rect) in result.positions.iter().enumerate() {
                let rect = rect.unwrap();
                assert_eq!(rect.w, g.members[i].size);
                assert!(rect.x >= 20.0 && rect.y - jumps[i] >= 20.0);
                assert!(rect.x + rect.w + 20.0 <= result.width as f32);
                assert!(rect.y + rect.h + 32.0 + 20.0 <= result.height as f32);
            }
        }
        g.layout = Layout::Row;
        let all = compose(&g, &[true; 3], &jumps);
        assert_eq!(all.width, 468);
        assert_eq!(all.height, 332);
        let fewer = compose(&g, &[true, false, true], &jumps);
        assert!(fewer.width < all.width && fewer.height < all.height);
        g.preserve_spaces = true;
        let preserved = compose(&g, &[true, false, true], &jumps);
        assert_eq!((preserved.width, preserved.height), (all.width, all.height));
        g.auto_size = false;
        let fixed = compose(&g, &[true; 3], &jumps);
        assert_eq!((fixed.width, fixed.height), (g.width, g.height));
    }
    #[test]
    fn free_auto_bounds_include_negative_positions_and_remain_stable_on_hide() {
        let mut g = Group::new("free");
        g.layout = Layout::Free;
        g.members = vec![
            Member {
                user: "1".into(),
                x: -140.0,
                y: -70.0,
                size: 100.0,
                ..Default::default()
            },
            Member {
                user: "2".into(),
                x: 240.0,
                y: 180.0,
                size: 80.0,
                ..Default::default()
            },
        ];
        let all = compose(&g, &[true, true], &[30.0, 10.0]);
        assert_eq!((all.width, all.height), (500, 432));
        let hidden = compose(&g, &[false, true], &[30.0, 10.0]);
        assert_eq!((hidden.width, hidden.height), (all.width, all.height));
        assert_eq!(hidden.positions[1].unwrap().x, all.positions[1].unwrap().x);
        for (i, rect) in all.positions.iter().enumerate() {
            let rect = rect.unwrap();
            assert_eq!(rect.x + all.origin_x, g.members[i].x);
            assert_eq!(rect.y + all.origin_y, g.members[i].y);
        }
    }
    #[test]
    fn legacy_groups_keep_fixed_dimensions_and_new_groups_save_auto_size() {
        let mut json = serde_json::to_value(Group::new("legacy")).unwrap();
        json.as_object_mut().unwrap().remove("auto_size");
        let legacy: Group = serde_json::from_value(json).unwrap();
        assert!(!legacy.auto_size);
        let new: Group =
            serde_json::from_value(serde_json::to_value(Group::new("new")).unwrap()).unwrap();
        assert!(new.auto_size);
    }
}
