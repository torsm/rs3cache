use crate::definitions::{mapsquares::GroupMapSquare, overlays::Overlay, underlays::Underlay};

use super::{
    mapcore::{INTERP, TILESIZE},
    tileshape::{OverlayShape, UnderlayShape},
};

use image::{GenericImage, Rgba, RgbaImage};

use std::{collections::HashMap, convert::TryInto};

/// Applies ground colouring to the base image.
pub fn put(
    plane: usize,
    img: &mut RgbaImage,
    squares: &GroupMapSquare,
    underlay_definitions: &HashMap<u32, Underlay>,
    overlay_definitions: &HashMap<u32, Overlay>,
) {
    if let Ok(columns) = squares.core().indexed_columns() {
        columns.for_each(|(column, (x, y))| {
            for p in plane..=3_usize {
                let condition: bool = unsafe {
                    (p == 0 && plane == 0)
                        || (p == plane && column.uget(1).settings.unwrap_or(0) & 0x2 == 0)
                        || (p == plane + 1 && (column.uget(1).settings.unwrap_or(0) & 0x2 != 0))
                        || (p >= plane && column.uget(0).settings.unwrap_or(0) & 0x2 != 0)
                        || (plane == 0 && column.uget(p).settings.unwrap_or(0) & 0x8 != 0)
                };

                if condition {
                    // Underlays
                    if let Some((red, green, blue)) = get_underlay_colour(underlay_definitions, &squares, p, x as usize, y as usize) {
                        let fill = Rgba([red, green, blue, 255u8]);

                        for (a, b) in UnderlayShape::new(column[p].shape, TILESIZE) {
                            unsafe {
                                debug_assert!(
                                    (TILESIZE * x + a) < img.width() && (TILESIZE * (63u32 - y) + b) < img.height(),
                                    "Index out of range."
                                );
                                img.unsafe_put_pixel(TILESIZE * x + a, TILESIZE * (63u32 - y) + b, fill)
                            }
                        }
                    }

                    // Overlays
                    if let Some(id) = column[p].overlay_id {
                        let ov = &overlay_definitions[&(id.checked_sub(1).expect("Not 100% sure about this invariant.") as u32)];
                        for colour in &[ov.primary_colour, ov.secondary_colour] {
                            match *colour {
                                Some((255, 0, 255)) => {}
                                Some((red, green, blue)) => {
                                    let fill = Rgba([red, green, blue, 255]);

                                    for (a, b) in OverlayShape::new(column[p].shape.unwrap_or(0), TILESIZE) {
                                        unsafe {
                                            debug_assert!(
                                                (TILESIZE * x + a) < img.width() && (TILESIZE * (63u32 - y) + b) < img.height(),
                                                "Index out of range."
                                            );

                                            img.unsafe_put_pixel(TILESIZE * x + a, TILESIZE * (63u32 - y) + b, fill)
                                        }
                                    }
                                }
                                None => {}
                            }
                        }
                    }
                }
            }
        })
    };
}

/// Averages out the [`Underlay`] colours, with a range specified by [`INTERP`].
fn get_underlay_colour(
    underlay_definitions: &HashMap<u32, Underlay>,
    squares: &GroupMapSquare,
    plane: usize,
    x: usize,
    y: usize,
) -> Option<(u8, u8, u8)> {
    // only compute a colour average if the tile has a underlay
    squares.core().get_tiles().unwrap()[(plane, x, y)].underlay_id.map(|_| {
        let tiles = squares.tiles_iter(plane, x, y, INTERP);

        let underlays = tiles.filter_map(|elem| elem.underlay_id);

        let colours = underlays.map(|id| {
            (
                1usize, /* weight, todo? */
                underlay_definitions[&(id.checked_sub(1).unwrap() as u32)].colour.unwrap(),
            )
        });

        let (weight, (reds, greens, blues)) = colours
            .map(|(w, (r, g, b))| (w, (r as usize * w, g as usize * w, b as usize * w)))
            .fold((0, (0, 0, 0)), |(acc_w, (acc_r, acc_g, acc_b)), (w, (r, g, b))| {
                (acc_w + w, (acc_r + r, acc_g + g, acc_b + b))
            });

        (
            (reds / weight).try_into().unwrap(),
            (greens / weight).try_into().unwrap(),
            (blues / weight).try_into().unwrap(),
        )
    })
}
