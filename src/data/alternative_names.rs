/*
* This list should be much more extensive, but it covers the main cases
* where Geonames fails to provide either localised or traditional / historical names
* for major cities in its name or toponymName attributes
* ideally this should be migrated to a full database
*/
pub const ALTERNATIVE_NAMES: [(&'static str, &'static str); 11] = [
  ("Madras", "Chennai"),
  ("Bombay", "Mumbai"),
  ("Brussel", "Brussels"),
  ("Bruxelles", "Brussels"),
  ("Calcutta", "Kolkata"),
  ("Lakhnau", "Lucknow"),
  ("Helsingfors", "Helsinki"),
  ("Venezia", "Venice"),
  ("Peking", "Beijing"),
  ("München", "Munich"),
  ("Muenchen", "Munich"),
];