//! NTRIP sourcetable parsing and mountpoint discovery.
//!
//! The sourcetable is a text document returned by NTRIP casters that lists
//! available data streams (mountpoints), their formats, and locations.

use std::fmt;

/// A parsed NTRIP sourcetable containing available streams.
#[derive(Debug, Clone, Default)]
pub struct Sourcetable {
    /// Stream entries (STR records)
    pub streams: Vec<StreamEntry>,
    /// Caster entries (CAS records)
    pub casters: Vec<CasterEntry>,
    /// Network entries (NET records)
    pub networks: Vec<NetworkEntry>,
}

/// A stream (mountpoint) entry from the sourcetable.
#[derive(Debug, Clone)]
pub struct StreamEntry {
    /// Mountpoint name
    pub mountpoint: String,
    /// Identifier/location description
    pub identifier: String,
    /// Data format (e.g., "RTCM 3.2", "RTCM 3.3")
    pub format: String,
    /// Format details (message types)
    pub format_details: String,
    /// Carrier phase info (0=No, 1=L1, 2=L1+L2)
    pub carrier: u8,
    /// Navigation system (e.g., "GPS+GLO+GAL+BDS")
    pub nav_system: String,
    /// Network name
    pub network: String,
    /// Country code (3-letter ISO)
    pub country: String,
    /// Latitude of reference station (degrees)
    pub latitude: f64,
    /// Longitude of reference station (degrees)
    pub longitude: f64,
    /// Whether NMEA GGA is required (0=No, 1=Yes)
    pub nmea_required: bool,
    /// Whether stream is generated from network (0=Single, 1=Network)
    pub is_network: bool,
    /// Generator software
    pub generator: String,
    /// Compression type
    pub compression: String,
    /// Authentication type (N=None, B=Basic, D=Digest)
    pub authentication: String,
    /// Fee required (N=No, Y=Yes)
    pub fee: bool,
    /// Bitrate in bits/second
    pub bitrate: u32,
    /// Miscellaneous info
    pub misc: String,
}

impl Default for StreamEntry {
    fn default() -> Self {
        Self {
            mountpoint: String::new(),
            identifier: String::new(),
            format: String::new(),
            format_details: String::new(),
            carrier: 0,
            nav_system: String::new(),
            network: String::new(),
            country: String::new(),
            latitude: 0.0,
            longitude: 0.0,
            nmea_required: false,
            is_network: false,
            generator: String::new(),
            compression: String::new(),
            authentication: String::new(),
            fee: false,
            bitrate: 0,
            misc: String::new(),
        }
    }
}

impl StreamEntry {
    /// Calculate distance to a reference position in kilometers.
    /// Uses Haversine formula for great-circle distance.
    pub fn distance_km(&self, lat: f64, lon: f64) -> f64 {
        const EARTH_RADIUS_KM: f64 = 6371.0;

        let lat1 = self.latitude.to_radians();
        let lat2 = lat.to_radians();
        let dlat = (lat - self.latitude).to_radians();
        let dlon = (lon - self.longitude).to_radians();

        let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();

        EARTH_RADIUS_KM * c
    }

    /// Check if this stream provides RTCM corrections.
    pub fn is_rtcm(&self) -> bool {
        self.format.to_uppercase().contains("RTCM")
    }
}

impl fmt::Display for StreamEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}) - {} @ ({:.4}, {:.4})",
            self.mountpoint, self.format, self.nav_system, self.latitude, self.longitude
        )
    }
}

/// A caster entry from the sourcetable.
#[derive(Debug, Clone, Default)]
pub struct CasterEntry {
    /// Caster hostname
    pub host: String,
    /// Caster port
    pub port: u16,
    /// Caster identifier
    pub identifier: String,
    /// Operator name
    pub operator: String,
    /// Whether NMEA is required
    pub nmea_required: bool,
    /// Country code
    pub country: String,
    /// Latitude
    pub latitude: f64,
    /// Longitude
    pub longitude: f64,
    /// Fallback host
    pub fallback_host: String,
    /// Fallback port
    pub fallback_port: u16,
    /// Miscellaneous info
    pub misc: String,
}

/// A network entry from the sourcetable.
#[derive(Debug, Clone, Default)]
pub struct NetworkEntry {
    /// Network identifier
    pub identifier: String,
    /// Network operator
    pub operator: String,
    /// Authentication type
    pub authentication: String,
    /// Fee required
    pub fee: bool,
    /// Web address
    pub web: String,
    /// Stream URL
    pub stream_url: String,
    /// Registration URL
    pub registration_url: String,
    /// Miscellaneous info
    pub misc: String,
}

impl Sourcetable {
    /// Parse a sourcetable from raw text.
    pub fn parse(text: &str) -> Self {
        let mut table = Sourcetable::default();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line == "ENDSOURCETABLE" {
                continue;
            }

            if line.starts_with("STR;") {
                if let Some(entry) = Self::parse_stream_entry(line) {
                    table.streams.push(entry);
                }
            } else if line.starts_with("CAS;") {
                if let Some(entry) = Self::parse_caster_entry(line) {
                    table.casters.push(entry);
                }
            } else if line.starts_with("NET;") {
                if let Some(entry) = Self::parse_network_entry(line) {
                    table.networks.push(entry);
                }
            }
        }

        table
    }

    /// Parse a STR (stream) entry.
    fn parse_stream_entry(line: &str) -> Option<StreamEntry> {
        let parts: Vec<&str> = line.split(';').collect();
        if parts.len() < 19 {
            return None;
        }

        Some(StreamEntry {
            mountpoint: parts[1].to_string(),
            identifier: parts[2].to_string(),
            format: parts[3].to_string(),
            format_details: parts[4].to_string(),
            carrier: parts[5].parse().unwrap_or(0),
            nav_system: parts[6].to_string(),
            network: parts[7].to_string(),
            country: parts[8].to_string(),
            latitude: parts[9].parse().unwrap_or(0.0),
            longitude: parts[10].parse().unwrap_or(0.0),
            nmea_required: parts[11] == "1",
            is_network: parts[12] == "1",
            generator: parts[13].to_string(),
            compression: parts[14].to_string(),
            authentication: parts[15].to_string(),
            fee: parts[16] == "Y",
            bitrate: parts[17].parse().unwrap_or(0),
            misc: parts.get(18).unwrap_or(&"").to_string(),
        })
    }

    /// Parse a CAS (caster) entry.
    fn parse_caster_entry(line: &str) -> Option<CasterEntry> {
        let parts: Vec<&str> = line.split(';').collect();
        if parts.len() < 12 {
            return None;
        }

        Some(CasterEntry {
            host: parts[1].to_string(),
            port: parts[2].parse().unwrap_or(2101),
            identifier: parts[3].to_string(),
            operator: parts[4].to_string(),
            nmea_required: parts[5] == "1",
            country: parts[6].to_string(),
            latitude: parts[7].parse().unwrap_or(0.0),
            longitude: parts[8].parse().unwrap_or(0.0),
            fallback_host: parts[9].to_string(),
            fallback_port: parts[10].parse().unwrap_or(0),
            misc: parts.get(11).unwrap_or(&"").to_string(),
        })
    }

    /// Parse a NET (network) entry.
    fn parse_network_entry(line: &str) -> Option<NetworkEntry> {
        let parts: Vec<&str> = line.split(';').collect();
        if parts.len() < 9 {
            return None;
        }

        Some(NetworkEntry {
            identifier: parts[1].to_string(),
            operator: parts[2].to_string(),
            authentication: parts[3].to_string(),
            fee: parts[4] == "Y",
            web: parts[5].to_string(),
            stream_url: parts[6].to_string(),
            registration_url: parts[7].to_string(),
            misc: parts.get(8).unwrap_or(&"").to_string(),
        })
    }

    /// Get all RTCM streams.
    pub fn rtcm_streams(&self) -> Vec<&StreamEntry> {
        self.streams.iter().filter(|s| s.is_rtcm()).collect()
    }

    /// Find streams sorted by distance to a reference position.
    pub fn streams_by_distance(&self, lat: f64, lon: f64) -> Vec<(&StreamEntry, f64)> {
        let mut streams: Vec<_> = self
            .streams
            .iter()
            .map(|s| (s, s.distance_km(lat, lon)))
            .collect();
        streams.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        streams
    }

    /// Find the nearest RTCM stream to a reference position.
    pub fn nearest_rtcm_stream(&self, lat: f64, lon: f64) -> Option<(&StreamEntry, f64)> {
        self.streams_by_distance(lat, lon)
            .into_iter()
            .find(|(s, _)| s.is_rtcm())
    }

    /// Find streams matching a mountpoint pattern (case-insensitive).
    pub fn find_streams(&self, pattern: &str) -> Vec<&StreamEntry> {
        let pattern_lower = pattern.to_lowercase();
        self.streams
            .iter()
            .filter(|s| s.mountpoint.to_lowercase().contains(&pattern_lower))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SOURCETABLE: &str = r#"CAS;rtk2go.com;2101;RTK2go;SNIP;0;USA;38.88;-77.02;;0;
NET;RTK2go;SNIP;B;N;http://www.rtk2go.com;none;none;
STR;ALIC00AUS0;Alice Springs;RTCM 3.2;1005(30),1077(1),1087(1),1097(1),1127(1),1230(1);2;GPS+GLO+GAL+BDS;GA;AUS;-23.6701;133.8855;0;0;SNIP;none;B;N;4800;
STR;BRIS00AUS0;Brisbane;RTCM 3.3;1005(30),1077(1),1087(1);2;GPS+GLO;GA;AUS;-27.4678;153.0281;1;0;SNIP;none;B;N;5000;
ENDSOURCETABLE
"#;

    #[test]
    fn test_parse_sourcetable() {
        let table = Sourcetable::parse(SAMPLE_SOURCETABLE);

        assert_eq!(table.casters.len(), 1);
        assert_eq!(table.networks.len(), 1);
        assert_eq!(table.streams.len(), 2);

        let alice = &table.streams[0];
        assert_eq!(alice.mountpoint, "ALIC00AUS0");
        assert_eq!(alice.identifier, "Alice Springs");
        assert!((alice.latitude - (-23.6701)).abs() < 0.001);
        assert!((alice.longitude - 133.8855).abs() < 0.001);
        assert!(alice.is_rtcm());
    }

    #[test]
    fn test_distance_calculation() {
        let entry = StreamEntry {
            latitude: -23.6701,
            longitude: 133.8855,
            ..Default::default()
        };

        // Distance to itself should be ~0
        let dist = entry.distance_km(-23.6701, 133.8855);
        assert!(dist < 0.001);

        // Distance to Brisbane (Alice Springs to Brisbane ~1900km)
        let dist = entry.distance_km(-27.4678, 153.0281);
        assert!(
            dist > 1800.0 && dist < 2100.0,
            "Expected ~1900km, got {}",
            dist
        );
    }

    #[test]
    fn test_nearest_stream() {
        let table = Sourcetable::parse(SAMPLE_SOURCETABLE);

        // From Brisbane, Brisbane should be nearest
        let nearest = table.nearest_rtcm_stream(-27.4678, 153.0281);
        assert!(nearest.is_some());
        let (stream, dist) = nearest.unwrap();
        assert_eq!(stream.mountpoint, "BRIS00AUS0");
        assert!(dist < 1.0); // Should be very close
    }

    #[test]
    fn test_find_streams() {
        let table = Sourcetable::parse(SAMPLE_SOURCETABLE);

        let results = table.find_streams("AUS");
        assert_eq!(results.len(), 2);

        let results = table.find_streams("BRIS");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].mountpoint, "BRIS00AUS0");
    }
}
