use geo_types::MultiPolygon;
use geojson::{Feature, Geometry, Value};
use num_traits::Float;
use osm_boundaries_utils::build_boundary;
use osmpbfreader::{OsmObj, OsmPbfReader, Relation};
use rstar::primitives::Rectangle;
use rstar::Envelope;
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use std::error::Error;
use std::fs::{write, File};

type Point2D = [f64; 2];

#[derive(Debug)]
struct Piece {
    rect: Rectangle<Point2D>,
    name: &'static str,
    area: f64,
}

impl Piece {
    pub fn new(lower: Point2D, upper: Point2D, name: &'static str) -> Self {
        let aabb = AABB::from_corners(lower, upper);
        let area = aabb.area();
        let rect = Rectangle::from_aabb(aabb);
        Piece { rect, name, area }
    }
}

impl RTreeObject for Piece {
    type Envelope = AABB<Point2D>;

    fn envelope(&self) -> Self::Envelope {
        self.rect.envelope()
    }
}

impl PointDistance for Piece {
    fn distance_2(&self, point: &Point2D) -> f64 {
        self.rect.distance_2(point)
    }
}

fn test_rtree() {
    let left_piece = Piece::new([0.0, 0.0], [0.4, 1.0], "left");
    let small_left_piece = Piece::new([0.0, 0.0], [0.3, 1.0], "small left");
    let right_piece = Piece::new([0.6, 0.0], [1.0, 1.0], "right");
    let middle_piece = Piece::new([0.25, 0.0], [0.75, 1.0], "middle");
    let huge_piece = Piece::new([0., 0.], [1.0, 1.0], "huge");

    let tree = RTree::<Piece>::bulk_load(vec![
        left_piece,
        small_left_piece,
        right_piece,
        middle_piece,
        huge_piece,
    ]);

    tree.locate_all_at_point(&[0.4, 0.5])
        .into_iter()
        .for_each(|p| {
            println!("piece: {:?}", p);
        });
}

fn is_admin(obj: &OsmObj) -> bool {
    get_admin(obj).is_some()
}

fn get_admin(obj: &OsmObj) -> Option<&Relation> {
    match obj {
        OsmObj::Relation(rel) => {
            if obj.tags().contains("boundary", "administrative")
                && obj.tags().contains("admin_level", "9")
            {
                Some(rel)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn to_geometry<T>(mp: &MultiPolygon<T>) -> Geometry
where
    T: Float,
{
    let value = Value::from(mp);
    Geometry::new(value)
}

fn to_feature(geometry: Geometry) -> Feature {
    Feature {
        bbox: None,
        geometry: Some(geometry),
        id: None,
        properties: None,
        foreign_members: None,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let file = File::open("bremen-latest.osm.pbf")?;
    // let file = File::open("berlin-latest.osm.pbf")?;
    let mut pbf = OsmPbfReader::new(file);
    let tuples = pbf.get_objs_and_deps(is_admin)?;

    let features = tuples
        .values()
        .filter_map(get_admin)
        .filter_map(|rel| {
            let name = rel.tags.get("name")?;
            let boundary = build_boundary(rel, &tuples)?;
            Some((name, boundary))
        })
        .map(|(_, boundary)| to_geometry(&boundary))
        .map(to_feature)
        .collect();

    let feature_collection = geojson::FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };

    write("output.geojson", feature_collection.to_string())?;

    // test_rtree();
    Ok(())
}
