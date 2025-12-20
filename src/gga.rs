//! GGA sentence generation for NTRIP position reporting.
//!
//! NTRIP casters often require periodic GGA position reports to select
//! the appropriate correction stream for the rover's location.

use chrono::{Timelike, Utc};

/// GGA sentence builder for NTRIP position reporting.
#[derive(Debug, Clone)]
pub struct GgaSentence {
    /// Latitude in degrees (positive = North)
    pub latitude: f64,
    /// Longitude in degrees (positive = East)
    pub longitude: f64,
    /// Altitude above mean sea level in meters
    pub altitude: f64,
    /// Fix quality (0=invalid, 1=GPS, 2=DGPS, 4=RTK fixed, 5=RTK float)
    pub quality: u8,
    /// Number of satellites in use
    pub num_satellites: u8,
    /// Horizontal dilution of precision
    pub hdop: f32,
}

impl GgaSentence {
    /// Create a new GGA sentence with the given position.
    pub fn new(latitude: f64, longitude: f64, altitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            altitude,
            quality: 1, // Default to GPS fix
            num_satellites: 10,
            hdop: 1.0,
        }
    }

    /// Set the fix quality.
    pub fn with_quality(mut self, quality: u8) -> Self {
        self.quality = quality;
        self
    }

    /// Set the number of satellites.
    pub fn with_satellites(mut self, num: u8) -> Self {
        self.num_satellites = num;
        self
    }

    /// Set the HDOP.
    pub fn with_hdop(mut self, hdop: f32) -> Self {
        self.hdop = hdop;
        self
    }

    /// Generate the NMEA GGA sentence string.
    ///
    /// Format: `$GPGGA,hhmmss.ss,llll.ll,a,yyyyy.yy,a,x,xx,x.x,x.x,M,x.x,M,x.x,xxxx*hh\r\n`
    pub fn to_nmea(&self) -> String {
        let now = Utc::now();
        let time = format!("{:02}{:02}{:02}.00", now.hour(), now.minute(), now.second());

        // Convert latitude to NMEA format (ddmm.mmmm)
        let (lat_str, lat_dir) = self.format_latitude();
        // Convert longitude to NMEA format (dddmm.mmmm)
        let (lon_str, lon_dir) = self.format_longitude();

        // Build the sentence without checksum
        let sentence = format!(
            "GPGGA,{},{},{},{},{},{},{:02},{:.1},{:.1},M,0.0,M,,",
            time,
            lat_str,
            lat_dir,
            lon_str,
            lon_dir,
            self.quality,
            self.num_satellites.min(99),
            self.hdop,
            self.altitude
        );

        // Calculate NMEA checksum (XOR of all bytes between $ and *)
        let checksum = sentence.bytes().fold(0u8, |acc, b| acc ^ b);

        format!("${}*{:02X}\r\n", sentence, checksum)
    }

    /// Format latitude as NMEA ddmm.mmmm string.
    fn format_latitude(&self) -> (String, char) {
        let dir = if self.latitude >= 0.0 { 'N' } else { 'S' };
        let lat_abs = self.latitude.abs();
        let degrees = lat_abs.floor() as u32;
        let minutes = (lat_abs - degrees as f64) * 60.0;
        (format!("{:02}{:07.4}", degrees, minutes), dir)
    }

    /// Format longitude as NMEA dddmm.mmmm string.
    fn format_longitude(&self) -> (String, char) {
        let dir = if self.longitude >= 0.0 { 'E' } else { 'W' };
        let lon_abs = self.longitude.abs();
        let degrees = lon_abs.floor() as u32;
        let minutes = (lon_abs - degrees as f64) * 60.0;
        (format!("{:03}{:07.4}", degrees, minutes), dir)
    }
}

impl Default for GgaSentence {
    fn default() -> Self {
        Self {
            latitude: 0.0,
            longitude: 0.0,
            altitude: 0.0,
            quality: 0,
            num_satellites: 0,
            hdop: 99.9,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gga_creation() {
        let gga = GgaSentence::new(-27.4698, 153.0251, 50.0);
        assert_eq!(gga.latitude, -27.4698);
        assert_eq!(gga.longitude, 153.0251);
        assert_eq!(gga.altitude, 50.0);
    }

    #[test]
    fn test_gga_builder() {
        let gga = GgaSentence::new(0.0, 0.0, 0.0)
            .with_quality(4)
            .with_satellites(15)
            .with_hdop(0.8);

        assert_eq!(gga.quality, 4);
        assert_eq!(gga.num_satellites, 15);
        assert_eq!(gga.hdop, 0.8);
    }

    #[test]
    fn test_latitude_format_north() {
        let gga = GgaSentence::new(27.4698, 0.0, 0.0);
        let (lat, dir) = gga.format_latitude();
        assert_eq!(dir, 'N');
        assert!(lat.starts_with("27")); // 27 degrees
    }

    #[test]
    fn test_latitude_format_south() {
        let gga = GgaSentence::new(-27.4698, 0.0, 0.0);
        let (lat, dir) = gga.format_latitude();
        assert_eq!(dir, 'S');
        assert!(lat.starts_with("27")); // 27 degrees (absolute)
    }

    #[test]
    fn test_longitude_format_east() {
        let gga = GgaSentence::new(0.0, 153.0251, 0.0);
        let (lon, dir) = gga.format_longitude();
        assert_eq!(dir, 'E');
        assert!(lon.starts_with("153")); // 153 degrees
    }

    #[test]
    fn test_longitude_format_west() {
        let gga = GgaSentence::new(0.0, -122.4194, 0.0);
        let (lon, dir) = gga.format_longitude();
        assert_eq!(dir, 'W');
        assert!(lon.starts_with("122")); // 122 degrees (absolute)
    }

    #[test]
    fn test_nmea_format() {
        let gga = GgaSentence::new(-27.4698, 153.0251, 50.0)
            .with_quality(4)
            .with_satellites(12);

        let nmea = gga.to_nmea();

        // Should start with $GPGGA
        assert!(nmea.starts_with("$GPGGA,"));
        // Should end with checksum and CRLF
        assert!(nmea.ends_with("\r\n"));
        // Should contain asterisk before checksum
        assert!(nmea.contains('*'));
        // Should have S for southern latitude
        assert!(nmea.contains(",S,"));
        // Should have E for eastern longitude
        assert!(nmea.contains(",E,"));
    }

    #[test]
    fn test_nmea_checksum() {
        let gga = GgaSentence::new(0.0, 0.0, 0.0);
        let nmea = gga.to_nmea();

        // Extract checksum from sentence
        let parts: Vec<&str> = nmea.trim().split('*').collect();
        assert_eq!(parts.len(), 2);

        let calculated_checksum = parts[0][1..].bytes().fold(0u8, |acc, b| acc ^ b);
        let reported_checksum = u8::from_str_radix(parts[1], 16).unwrap();

        assert_eq!(calculated_checksum, reported_checksum);
    }
}
