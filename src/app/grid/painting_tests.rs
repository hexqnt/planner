use egui::epaint::{Mesh, TessellationOptions, Tessellator};

use super::*;

#[test]
fn range_fills_have_consistent_triangle_winding_at_fractional_scales_and_offsets() {
    let calendar = Calendar::new(
        crate::model::Year::try_from(2026).unwrap(),
        crate::calendar::Region::Federal,
    );
    let month = &calendar.months[10];
    let color = event_color([255, 215, 0], true);
    let ctx = egui::Context::default();
    for scale in [1.0, 1.1, 1.25, 1.5, 1.75, 2.0] {
        for offset in [0.0, 0.25, 0.4, 0.5, 0.75] {
            let origin = egui::pos2(409.0 + offset, 140.0 + offset);
            for costs in [false, true] {
                for stroke in [None, Some(Stroke::new(config::SELECTION_STROKE, BLUE))] {
                    for (start, end) in [
                        ("2026-11-11", "2026-11-11"),
                        ("2026-11-11", "2026-11-12"),
                        ("2026-11-08", "2026-11-09"),
                        ("2026-11-01", "2026-11-30"),
                    ] {
                        let range =
                            DateRange::new(start.parse().unwrap(), end.parse().unwrap()).unwrap();
                        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                            paint_range(
                                ui,
                                month,
                                range,
                                origin,
                                color,
                                stroke,
                                Geometry::new(costs),
                            );
                        });
                        let mut fill_triangles = 0;
                        for shape in &output.shapes {
                            let mut mesh = Mesh::default();
                            Tessellator::new(
                                scale,
                                TessellationOptions::default(),
                                [1, 1],
                                Vec::new(),
                            )
                            .tessellate_shape(shape.shape.clone(), &mut mesh);
                            assert!(mesh.is_valid());
                            let mut min_area = f32::INFINITY;
                            let mut max_area = f32::NEG_INFINITY;
                            for indices in mesh.indices.as_chunks::<3>().0 {
                                let [a, b, c] = indices
                                    .map(|index| mesh.vertices[usize::try_from(index).unwrap()]);
                                if [a, b, c].iter().all(|vertex| vertex.color == color) {
                                    let ab = b.pos - a.pos;
                                    let ac = c.pos - a.pos;
                                    let area = ab.x.mul_add(ac.y, -ab.y * ac.x);
                                    assert!(area.is_finite());
                                    min_area = min_area.min(area);
                                    max_area = max_area.max(area);
                                    fill_triangles += 1;
                                }
                            }
                            // Разная ориентация треугольников заливки означает самопересечение и повторное смешивание цвета.
                            assert!(
                                min_area >= -0.01 || max_area <= 0.01,
                                "Overlapping fill triangles: scale={scale}, offset={offset}, costs={costs}, stroke={stroke:?}, range={start}..={end}, areas={min_area}..={max_area}"
                            );
                        }
                        assert!(fill_triangles > 0);
                        output.drop_without_applying_deltas();
                    }
                }
            }
        }
    }
}
