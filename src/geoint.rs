//! GEOINT — geospatial intelligence. The first discipline module: it turns
//! geography into a correlation signal. Entities that sit within a radius of each
//! other are linked `co_located_with` (a spatial analogue of the shared-hub
//! correlator), so proximity becomes structure that risk, network science and the
//! map all use. Deterministic and offline; radius via `CORTEX_GEO_RADIUS_KM`.

use crate::ontology::{KnowledgeGraph, Relationship};

/// Parse a lat/lon from an entity's attributes across common key spellings.
pub fn geo_of(attrs: &indexmap::IndexMap<String, String>) -> Option<(f64, f64)> {
    let get = |names: &[&str]| -> Option<f64> {
        for (k, v) in attrs {
            let lk: String = k.to_lowercase().chars().filter(|c| c.is_ascii_alphabetic()).collect();
            if names.contains(&lk.as_str()) {
                if let Some(f) = v.split(',').next().and_then(|s| s.trim().parse::<f64>().ok()) {
                    return Some(f);
                }
            }
        }
        None
    };
    let mut lat = get(&["lat", "latitude", "latitudeapprox", "gpslatitude"]);
    let mut lon = get(&["lon", "lng", "longitude", "longitudeapprox", "gpslongitude"]);
    // "lat,lon" packed into a single field.
    if lat.is_none() {
        for (_, v) in attrs {
            let parts: Vec<&str> = v.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(a), Ok(b)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
                    if a.abs() <= 90.0 && b.abs() <= 180.0 {
                        lat = Some(a);
                        lon = Some(b);
                        break;
                    }
                }
            }
        }
    }
    match (lat, lon) {
        (Some(a), Some(b)) if a.abs() <= 90.0 && b.abs() <= 180.0 => Some((a, b)),
        _ => None,
    }
}

/// Great-circle distance in kilometres (haversine).
pub fn haversine_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    let r = 6371.0088;
    let (la1, lo1) = (a.0.to_radians(), a.1.to_radians());
    let (la2, lo2) = (b.0.to_radians(), b.1.to_radians());
    let dla = la2 - la1;
    let dlo = lo2 - lo1;
    let h = (dla / 2.0).sin().powi(2) + la1.cos() * la2.cos() * (dlo / 2.0).sin().powi(2);
    2.0 * r * h.sqrt().asin()
}

#[derive(Debug, Default)]
pub struct GeoStats {
    pub geolocated: usize,
    pub colocations: usize,
}

/// Add `co_located_with` edges between geolocated entities within the radius.
/// Skips entities with too many neighbours (a dense cluster is already a hub, and
/// O(n²) noise helps no one) — the same discipline as the shared-hub correlator.
pub fn correlate_geo(graph: &mut KnowledgeGraph) -> GeoStats {
    let radius_km: f64 = std::env::var("CORTEX_GEO_RADIUS_KM").ok().and_then(|s| s.parse().ok()).unwrap_or(5.0);
    // Collect (id, lat, lon) for geolocated entities; cap for very large graphs.
    let mut pts: Vec<(String, (f64, f64))> = graph
        .entities
        .iter()
        .filter_map(|(id, e)| geo_of(&e.attributes).map(|c| (id.clone(), c)))
        .collect();
    let mut stats = GeoStats { geolocated: pts.len(), colocations: 0 };
    if pts.len() < 2 || pts.len() > 4000 {
        if pts.len() > 4000 { pts.truncate(4000); } else { return stats; }
    }

    // Skip pairs that are already directly connected — those are same-record
    // relations (a report and its account share coordinates), not the GEOINT
    // signal. The value is proximity between OTHERWISE-unconnected entities.
    let connected: std::collections::HashSet<(String, String)> = graph
        .relationships
        .iter()
        .map(|r| {
            if r.source_id <= r.target_id { (r.source_id.clone(), r.target_id.clone()) }
            else { (r.target_id.clone(), r.source_id.clone()) }
        })
        .collect();
    let is_connected = |a: &str, b: &str| {
        let key = if a <= b { (a.to_string(), b.to_string()) } else { (b.to_string(), a.to_string()) };
        connected.contains(&key)
    };

    // Pairwise within radius; bbox pre-reject keeps it cheap.
    // ~111 km per degree of latitude; be generous on the longitude gate.
    let deg = radius_km / 111.0 + 0.001;
    let mut neighbors: Vec<Vec<usize>> = vec![Vec::new(); pts.len()];
    for i in 0..pts.len() {
        for j in (i + 1)..pts.len() {
            if (pts[i].1 .0 - pts[j].1 .0).abs() > deg * 1.5 {
                continue;
            }
            if is_connected(&pts[i].0, &pts[j].0) {
                continue;
            }
            if haversine_km(pts[i].1, pts[j].1) <= radius_km {
                neighbors[i].push(j);
                neighbors[j].push(i);
            }
        }
    }
    let mut edges: Vec<Relationship> = Vec::new();
    for i in 0..pts.len() {
        if neighbors[i].len() > 12 {
            continue; // dense area — don't draw the full clique
        }
        for &j in &neighbors[i] {
            if i < j {
                let mut r = Relationship::new(pts[i].0.clone(), "co_located_with", pts[j].0.clone(), 0.6);
                let d = haversine_km(pts[i].1, pts[j].1);
                r.source_reference = Some(format!("geo:{:.1}km", d));
                edges.push(r);
            }
        }
    }
    for e in edges {
        let before = graph.relationship_count();
        graph.add_relationship(e);
        if graph.relationship_count() > before {
            stats.colocations += 1;
        }
    }
    stats
}
