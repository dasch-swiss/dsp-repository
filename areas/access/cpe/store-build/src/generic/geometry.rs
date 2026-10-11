//! An annotation's geometry: the archive's JSON, as the box and the points the store holds.
//!
//! The box is in fractions of the image, from its top left corner. The arithmetic is the store's
//! own, operation for operation: another order gives another last digit.

use serde_json::Value;

use crate::store::{Real, RealError};

/// The slack of the box check. `cpe-core`'s check of a store's annotations (`MalformedAnnotation`)
/// allows the same (`ANNOTATION_TOLERANCE` in `areas/access/cpe/core/src/validate.rs`), and a
/// test here holds the two equal: a box this module passes and that check refuses is a built
/// store that does not validate.
const TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    Rectangle,
    Polygon,
    Circle,
}

impl Shape {
    /// The word the archive's JSON and the store's `shape` column both use.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Shape::Rectangle => "rectangle",
            Shape::Polygon => "polygon",
            Shape::Circle => "circle",
        }
    }

    fn named(name: &str) -> Option<Self> {
        [Shape::Rectangle, Shape::Polygon, Shape::Circle]
            .into_iter()
            .find(|shape| shape.name() == name)
    }

    fn takes(self, points: usize) -> bool {
        match self {
            Shape::Rectangle => points == 2,
            Shape::Polygon => points >= 3,
            Shape::Circle => points == 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Geometry {
    pub(crate) shape: Shape,
    pub(crate) x: Real,
    pub(crate) y: Real,
    pub(crate) w: Real,
    pub(crate) h: Real,
    /// A polygon's vertices in the archive's order, each `x,y`, joined by one space. `None` for
    /// a rectangle and a circle, which their box describes.
    pub(crate) points: Option<String>,
}

/// Why a geometry was refused. A box is quoted as its numbers would have been written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum GeometryError {
    #[error("not a JSON object with a list of points, each an x and a y, and for a circle a radius")]
    NotRead,
    #[error("unknown shape {0:?}")]
    UnknownShape(String),
    #[error("a {} of {points} points", shape.name())]
    PointCount { shape: Shape, points: usize },
    #[error("the box {0} has no area")]
    EmptyBox(String),
    #[error("the box {0} is not inside the image")]
    OutsideImage(String),
    #[error(transparent)]
    Number(#[from] RealError),
}

/// The box and points of the archive's geometry JSON.
///
/// A rectangle is its two corners, in either order; a polygon's box is the bounding box of its
/// vertices; a circle's box is its centre less and plus its radius on each axis, so the store's
/// shape is the ellipse inscribed in the box. The JSON's line colour, line width and status are
/// not read: an annotation's colour is a value of its own.
pub(crate) fn normalise(geometry: &str) -> Result<Geometry, GeometryError> {
    let json: Value = serde_json::from_str(geometry).map_err(|_| GeometryError::NotRead)?;
    let points = json
        .get("points")
        .and_then(Value::as_array)
        .and_then(|points| points.iter().map(point).collect::<Option<Vec<_>>>())
        .ok_or(GeometryError::NotRead)?;
    let name = json.get("type").and_then(Value::as_str).unwrap_or_default();
    let shape = Shape::named(name).ok_or_else(|| GeometryError::UnknownShape(name.to_string()))?;
    if !shape.takes(points.len()) {
        return Err(GeometryError::PointCount { shape, points: points.len() });
    }

    let [x, y, w, h] = match (shape, points.as_slice()) {
        (Shape::Circle, &[(cx, cy)]) => {
            let (rx, ry) = json.get("radius").and_then(point).ok_or(GeometryError::NotRead)?;
            let (rx, ry) = (rx.abs(), ry.abs());
            [cx - rx, cy - ry, 2.0 * rx, 2.0 * ry]
        }
        _ => {
            let x = points.iter().map(|&(x, _)| x);
            let y = points.iter().map(|&(_, y)| y);
            let (x0, y0) = (x.clone().fold(f64::INFINITY, f64::min), y.clone().fold(f64::INFINITY, f64::min));
            let (x1, y1) = (x.fold(f64::NEG_INFINITY, f64::max), y.fold(f64::NEG_INFINITY, f64::max));
            [x0, y0, x1 - x0, y1 - y0]
        }
    };
    let quoted = || format!("x={x:?} y={y:?} w={w:?} h={h:?}");
    if !(w > 0.0 && h > 0.0) {
        return Err(GeometryError::EmptyBox(quoted()));
    }
    let inside = x >= -TOLERANCE && y >= -TOLERANCE && x + w <= 1.0 + TOLERANCE && y + h <= 1.0 + TOLERANCE;
    if !inside {
        return Err(GeometryError::OutsideImage(quoted()));
    }

    let vertices = match shape {
        Shape::Polygon => Some(
            points
                .iter()
                .map(|&(x, y)| Ok(format!("{},{}", Real::new(x)?.as_str(), Real::new(y)?.as_str())))
                .collect::<Result<Vec<_>, RealError>>()?
                .join(" "),
        ),
        Shape::Rectangle | Shape::Circle => None,
    };
    Ok(Geometry {
        shape,
        x: Real::new(x)?,
        y: Real::new(y)?,
        w: Real::new(w)?,
        h: Real::new(h)?,
        points: vertices,
    })
}

/// The `x` and `y` of a JSON object, both numbers.
fn point(value: &Value) -> Option<(f64, f64)> {
    Some((value.get("x")?.as_f64()?, value.get("y")?.as_f64()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (shape, x, y, w, h, points) as texts.
    fn normalised(geometry: &str) -> (&'static str, [String; 4], Option<String>) {
        let found = normalise(geometry).expect("the geometry is accepted");
        let numbers = [found.x, found.y, found.w, found.h].map(|real| real.as_str().to_string());
        (found.shape.name(), numbers, found.points)
    }

    fn numbers(expected: [&str; 4]) -> [String; 4] {
        expected.map(str::to_string)
    }

    // The expected numbers of the four tests below are `python3`'s, from the same operations.

    const RECTANGLE_BOX: [&str; 4] = [
        "0.17034700315457413",
        "0.19907407407407407",
        "0.6498422712933754",
        "0.6620370370370371",
    ];

    #[test]
    fn test_normalise_rectangle_is_the_box_of_its_two_corners() {
        let geometry = r##"{"status":"active","lineColor":"#ff3333","lineWidth":2,"points":[{"x":0.17034700315457413,"y":0.19907407407407407},{"x":0.8201892744479495,"y":0.8611111111111112}],"type":"rectangle","original_index":0}"##;

        assert_eq!(normalised(geometry), ("rectangle", numbers(RECTANGLE_BOX), None));
    }

    #[test]
    fn test_normalise_rectangle_stored_bottom_right_first_gives_the_same_box() {
        let geometry = r#"{"points":[{"x":0.8201892744479495,"y":0.8611111111111112},{"x":0.17034700315457413,"y":0.19907407407407407}],"type":"rectangle"}"#;

        assert_eq!(normalised(geometry), ("rectangle", numbers(RECTANGLE_BOX), None));
    }

    #[test]
    fn test_normalise_polygon_is_its_bounding_box_and_its_vertices_in_order() {
        let geometry = r#"{"points":[{"x":0.17252396166134185,"y":0.1597222222222222},{"x":0.8242811501597445,"y":0.14583333333333334},{"x":0.8242811501597445,"y":0.8310185185185185},{"x":0.1757188498402556,"y":0.8240740740740741},{"x":0.1757188498402556,"y":0.1597222222222222},{"x":0.16932907348242812,"y":0.16435185185185186}],"type":"polygon"}"#;

        assert_eq!(
            normalised(geometry),
            (
                "polygon",
                numbers([
                    "0.16932907348242812",
                    "0.14583333333333334",
                    "0.6549520766773164",
                    "0.6851851851851851",
                ]),
                Some(
                    "0.17252396166134185,0.1597222222222222 0.8242811501597445,0.14583333333333334 \
                     0.8242811501597445,0.8310185185185185 0.1757188498402556,0.8240740740740741 \
                     0.1757188498402556,0.1597222222222222 0.16932907348242812,0.16435185185185186"
                        .to_string()
                ),
            )
        );
    }

    #[test]
    fn test_normalise_circle_is_the_box_around_its_centre_and_radius() {
        let geometry = r#"{"points":[{"x":0.3400735294117647,"y":0.45376078914919854}],"type":"circle","radius":{"x":0.04595588235294118,"y":0.03082614056720101}}"#;

        assert_eq!(
            normalised(geometry),
            (
                "circle",
                numbers([
                    "0.29411764705882354",
                    "0.42293464858199753",
                    "0.09191176470588236",
                    "0.06165228113440202",
                ]),
                None,
            )
        );
    }

    #[test]
    fn test_normalise_circle_with_a_negative_radius_has_the_same_box() {
        let geometry =
            |radius: &str| format!(r#"{{"points":[{{"x":0.5,"y":0.5}}],"type":"circle","radius":{radius}}}"#);

        assert_eq!(
            normalise(&geometry(r#"{"x":-0.25,"y":-0.125}"#)),
            normalise(&geometry(r#"{"x":0.25,"y":0.125}"#))
        );
    }

    #[test]
    fn test_tolerance_is_the_one_the_engine_checks_a_store_with() {
        let validator = include_str!("../../../core/src/validate.rs");
        let line = format!("const ANNOTATION_TOLERANCE: f64 = {TOLERANCE:?};");

        assert!(
            validator.contains(&line),
            "areas/access/cpe/core/src/validate.rs has no line `{line}`. \
             That constant and this module's TOLERANCE must be one number: if the engine's value \
             changed, change TOLERANCE; if the constant moved, point this test at its new file. \
             Do not loosen the test."
        );
    }

    fn refusal(geometry: &str) -> GeometryError {
        normalise(geometry).expect_err("the geometry is refused")
    }

    #[test]
    fn test_normalise_text_that_is_no_geometry_is_not_read() {
        let cases = [
            "",
            "[]",
            r#"{"type":"rectangle"}"#,
            r#"{"type":"rectangle","points":{"x":0.1,"y":0.1}}"#,
            r#"{"type":"rectangle","points":[{"x":0.1},{"x":0.5,"y":0.5}]}"#,
            r#"{"type":"rectangle","points":[{"x":"0.1","y":0.1},{"x":0.5,"y":0.5}]}"#,
            r#"{"type":"circle","points":[{"x":0.5,"y":0.5}]}"#,
            r#"{"type":"circle","points":[{"x":0.5,"y":0.5}],"radius":0.1}"#,
        ];

        for geometry in cases {
            assert_eq!(refusal(geometry), GeometryError::NotRead, "{geometry}");
        }
    }

    #[test]
    fn test_normalise_unknown_shape_is_refused_by_name() {
        let geometry = r#"{"type":"ellipse","points":[{"x":0.1,"y":0.1},{"x":0.5,"y":0.5}]}"#;
        let untyped = r#"{"points":[{"x":0.1,"y":0.1},{"x":0.5,"y":0.5}]}"#;

        assert_eq!(refusal(geometry), GeometryError::UnknownShape("ellipse".to_string()));
        assert_eq!(refusal(untyped), GeometryError::UnknownShape(String::new()));
    }

    #[test]
    fn test_normalise_wrong_point_count_is_refused_for_each_shape() {
        let with_points = |shape: &str, count: usize| {
            let points = vec![r#"{"x":0.25,"y":0.25}"#; count].join(",");
            format!(r#"{{"type":"{shape}","points":[{points}],"radius":{{"x":0.1,"y":0.1}}}}"#)
        };
        let cases = [
            (Shape::Rectangle, 1),
            (Shape::Rectangle, 3),
            (Shape::Polygon, 2),
            (Shape::Circle, 0),
            (Shape::Circle, 2),
        ];

        for (shape, points) in cases {
            assert_eq!(
                refusal(&with_points(shape.name(), points)),
                GeometryError::PointCount { shape, points }
            );
        }
    }

    fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> String {
        format!(r#"{{"type":"rectangle","points":[{{"x":{x0:?},"y":{y0:?}}},{{"x":{x1:?},"y":{y1:?}}}]}}"#)
    }

    #[test]
    fn test_normalise_box_without_area_is_refused() {
        for geometry in [rectangle(0.25, 0.25, 0.25, 0.5), rectangle(0.25, 0.5, 0.75, 0.5)] {
            assert!(matches!(refusal(&geometry), GeometryError::EmptyBox(_)), "{geometry}");
        }
    }

    #[test]
    fn test_normalise_box_outside_the_image_is_refused_on_each_edge() {
        for geometry in [
            rectangle(-0.01, 0.25, 0.5, 0.5),
            rectangle(0.25, -0.01, 0.5, 0.5),
            rectangle(0.25, 0.25, 1.01, 0.5),
            rectangle(0.25, 0.25, 0.5, 1.01),
        ] {
            assert!(matches!(refusal(&geometry), GeometryError::OutsideImage(_)), "{geometry}");
        }
    }

    #[test]
    fn test_normalise_box_over_the_edge_by_less_than_the_slack_is_not_refused_for_its_place() {
        let within_the_slack = rectangle(-1e-10, 0.0, 1.0, 1.0 + 1e-10);

        // Refused for how the number is written, so not for where the box is.
        assert!(matches!(refusal(&within_the_slack), GeometryError::Number(_)));
        assert!(normalise(&rectangle(0.0, 0.0, 1.0, 1.0)).is_ok());
    }

    #[test]
    fn test_normalise_number_written_with_an_exponent_is_refused() {
        let geometry = rectangle(0.00001, 0.25, 0.5, 0.5);

        assert_eq!(
            refusal(&geometry),
            GeometryError::Number(RealError::Exponent("1e-5".to_string()))
        );
    }
}
