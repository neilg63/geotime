/*
* This list should be much more extensive, but it covers the main cases discovered so far
* in which Geonames fails to provide localised or traditional / historical name variants
* for major cities in its name or toponymName attributes.
* The latter attribute usually contains the official transcribed name, 
* but sometimes is only available in international English even if the variant is common locally. 
* This lookup set serves only for post-filtering as geonames usually picks up such variants from the search query, 
* but does not include them in the results, e.f. q=Madras will match Chennai, 
* but this will not appear in the results for capital of Tamil Nadu.
* Should this data set grow beyond a few hundred entries, it should be migrated to a full database.
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