//! "Potentiate" stage: enrich extracted entities with normalized/derived
//! attributes and materialize derived hub entities so correlation has more to
//! link. Deterministic and offline; every derived attribute/entity/edge is
//! provenance-tagged (entities get source `derived:enrich`, entities carry the
//! `enriched` tag). Runs after extraction, before correlation.
//!
//! The highest-leverage move: extraction turns a URL into a `Url` node but never
//! derives its registrable domain, so URLs of the same site don't share a hub.
//! Here we derive that domain (and lift sub-domains to their registrable parent),
//! materialize it as a `Domain` node, and link to it — which lets the shared-hub
//! correlator draw `shares_domain_with` between everything on the same site.

use crate::ontology::{Entity, EntityKind, KnowledgeGraph, Relationship};

/// What the enrichment pass produced (for the audit line / progress output).
#[derive(Debug, Default)]
pub struct EnrichStats {
    pub attrs: usize,
    pub entities: usize,
    pub edges: usize,
    /// Entities whose file hash matched an integrated reference source.
    pub ref_matches: usize,
}

/// Enrich the graph in place. Returns counts of what was derived.
pub fn enrich(graph: &mut KnowledgeGraph) -> EnrichStats {
    let e0 = graph.entity_count();
    let r0 = graph.relationship_count();
    let mut attrs = 0usize;

    // Pass 1 — derive attributes on existing nodes; collect domain links to add.
    let ids: Vec<String> = graph.entities.keys().cloned().collect();
    let mut domain_links: Vec<(String, String)> = Vec::new(); // (entity_id, registrable_domain)

    for id in &ids {
        let (kind, label, ts) = {
            let e = graph.entities.get(id).unwrap();
            let ts = e
                .attributes
                .get("timestamp")
                .or_else(|| e.attributes.get("created_at"))
                .or_else(|| e.attributes.get("received_at"))
                .or_else(|| e.attributes.get("observed_at"))
                .cloned();
            (e.kind, e.label.clone(), ts)
        };
        let e = graph.entities.get_mut(id).unwrap();

        match kind {
            EntityKind::Url => {
                if let Some(host) = url_host(&label) {
                    attrs += set_attr(e, "host", &host);
                    let reg = registrable_domain(&host);
                    if !reg.is_empty() {
                        attrs += set_attr(e, "registrable_domain", &reg);
                        domain_links.push((id.clone(), reg));
                    }
                }
            }
            EntityKind::Domain => {
                let low = label.trim().to_lowercase();
                let reg = registrable_domain(&low);
                // Only lift genuine sub-domains (a.b.c) up to their parent (b.c).
                if !reg.is_empty() && reg != low {
                    attrs += set_attr(e, "registrable_domain", &reg);
                    domain_links.push((id.clone(), reg));
                }
            }
            EntityKind::Ip => {
                attrs += set_attr(e, "ip_scope", ip_scope(&label));
            }
            _ => {}
        }

        // Temporal: behavioural window from any timestamp-ish attribute.
        if let Some(ts) = ts {
            if let Some(hour) = iso_hour(&ts) {
                let e = graph.entities.get_mut(id).unwrap();
                attrs += set_attr(e, "activity_hour", &hour);
            }
        }
    }

    // Pass 2 — materialize the registrable-domain hub and link to it. upsert dedups
    // against any Domain that extraction already created, so no duplicate nodes.
    for (eid, reg) in domain_links {
        if !reg.contains('.') {
            continue;
        }
        let dom = Entity::new(EntityKind::Domain, &reg)
            .with_source("derived:enrich");
        let dom_id = graph.upsert_entity(dom);
        if let Some(d) = graph.entities.get_mut(&dom_id) {
            if !d.tags.iter().any(|t| t == "enriched") {
                d.tags.push("enriched".into());
            }
        }
        if dom_id != eid {
            let mut r = Relationship::new(eid.clone(), "belongs_to_domain", dom_id.clone(), 0.9);
            r.source_reference = Some("derived:enrich".into());
            graph.add_relationship(r);
        }
    }

    // Pass 3 — match file hashes against integrated reference sources (known-bad
    // hash feeds, known-CSAM sets, watchlists). A hit tags the entity with the
    // reference severity (which risk scoring picks up) and its provenance.
    let refs = crate::references::load();
    let mut ref_matches = 0usize;
    if !refs.is_empty() {
        // Perceptual near-duplicate threshold (bits). Conservative default; a
        // recompressed/resized copy typically stays within a handful of bits.
        let max_dist: u32 = std::env::var("CORTEX_PHASH_MAXDIST").ok().and_then(|s| s.parse().ok()).unwrap_or(10);
        let ids: Vec<String> = graph.entities.keys().cloned().collect();
        for id in ids {
            let (file_hash, phash) = {
                let e = graph.entities.get(&id).unwrap();
                (e.attributes.get("file_hash").cloned(), e.attributes.get("perceptual_hash").cloned())
            };

            // Exact file-hash match (identical bytes).
            if let Some(hit) = file_hash.as_deref().and_then(|h| refs.lookup(h)) {
                let (source, category, severity) = (hit.source.clone(), hit.category.clone(), hit.severity.clone());
                let e = graph.entities.get_mut(&id).unwrap();
                attrs += set_attr(e, "ref_source", &source);
                attrs += set_attr(e, "ref_category", &category);
                attrs += set_attr(e, "ref_severity", &severity);
                attrs += set_attr(e, "ref_match", "exact");
                mark_ref(e, &source, &severity, "known-file-hash");
                ref_matches += 1;
                continue; // exact wins; don't also perceptual-match the same node
            }

            // Perceptual near-duplicate match (altered/recompressed image).
            if let Some(m) = phash.as_deref().and_then(|p| refs.lookup_perceptual(p, max_dist)) {
                let (source, category, severity) = (m.hit.source.clone(), m.hit.category.clone(), m.hit.severity.clone());
                let e = graph.entities.get_mut(&id).unwrap();
                attrs += set_attr(e, "ref_source", &source);
                attrs += set_attr(e, "ref_category", &category);
                attrs += set_attr(e, "ref_severity", &severity);
                attrs += set_attr(e, "ref_match", "perceptual");
                attrs += set_attr(e, "ref_similarity", &format!("{:.4}", m.similarity));
                attrs += set_attr(e, "perceptual_distance", &format!("{}/{}", m.distance, m.bits));
                mark_ref(e, &source, &severity, "known-image-match");
                ref_matches += 1;
            }
        }
    }

    EnrichStats {
        attrs,
        entities: graph.entity_count() - e0,
        edges: graph.relationship_count() - r0,
        ref_matches,
    }
}

/// Tag a reference-source match: severity token (feeds risk scoring), a match
/// tag (provenance) and the source in the entity's source list.
fn mark_ref(e: &mut Entity, source: &str, severity: &str, match_tag: &str) {
    for tag in [match_tag, severity] {
        if !tag.is_empty() && !e.tags.iter().any(|t| t == tag) {
            e.tags.push(tag.to_string());
        }
    }
    let src = format!("ref:{source}");
    if !e.sources.contains(&src) {
        e.sources.push(src);
    }
}

/// Insert an attribute only if absent; returns 1 if it was added, else 0. Also
/// marks the entity `enriched` so the derivation is visible/auditable.
fn set_attr(e: &mut Entity, k: &str, v: &str) -> usize {
    if v.is_empty() || e.attributes.contains_key(k) {
        return 0;
    }
    e.attributes.insert(k.to_string(), v.to_string());
    if !e.tags.iter().any(|t| t == "enriched") {
        e.tags.push("enriched".into());
    }
    1
}

/// Extract the host from a URL label, tolerating malformed doubled schemes
/// (e.g. "https://https://cdn.example.net/f").
fn url_host(url: &str) -> Option<String> {
    let mut s = url.trim();
    loop {
        let lower = s.to_lowercase();
        if let Some(rest) = lower
            .strip_prefix("https://")
            .or_else(|| lower.strip_prefix("http://"))
        {
            s = &s[s.len() - rest.len()..];
        } else if let Some(rest) = lower.strip_prefix("www.") {
            s = &s[s.len() - rest.len()..];
        } else {
            break;
        }
    }
    // host ends at the first '/', ':', '?', or '#'.
    let host: String = s
        .chars()
        .take_while(|c| !matches!(c, '/' | ':' | '?' | '#'))
        .collect::<String>()
        .trim()
        .to_lowercase();
    if host.contains('.') && !host.is_empty() {
        Some(host)
    } else {
        None
    }
}

/// Naive registrable domain (eTLD+1): last two dot-labels. Good enough offline
/// for the common case; a public-suffix list would refine multi-part TLDs.
fn registrable_domain(host: &str) -> String {
    let host = host.trim().trim_end_matches('.').to_lowercase();
    let labels: Vec<&str> = host.split('.').filter(|l| !l.is_empty()).collect();
    if labels.len() < 2 {
        return String::new();
    }
    labels[labels.len() - 2..].join(".")
}

/// Classify an IPv4 as private / loopback / public — a cheap correlation and
/// risk signal (internal vs internet-facing).
fn ip_scope(ip: &str) -> &'static str {
    let o: Vec<u16> = ip.split('.').filter_map(|p| p.parse().ok()).collect();
    if o.len() != 4 {
        return "unknown";
    }
    match (o[0], o[1]) {
        (127, _) => "loopback",
        (10, _) => "private",
        (192, 168) => "private",
        (172, b) if (16..=31).contains(&b) => "private",
        (169, 254) => "link-local",
        _ => "public",
    }
}

/// Best-effort hour-of-day from an ISO-ish timestamp ("...T13:..." or " 13:").
fn iso_hour(ts: &str) -> Option<String> {
    let t = ts.trim();
    let after = t.split(['T', ' ']).nth(1)?;
    let hh: String = after.chars().take(2).collect();
    if hh.len() == 2 && hh.chars().all(|c| c.is_ascii_digit()) {
        let h: u8 = hh.parse().ok()?;
        if h < 24 {
            return Some(format!("{:02}", h));
        }
    }
    None
}
